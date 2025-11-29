//! Google Fonts source.

use crate::cli::CliContext;
use crate::config::{config, util, util::os_to_url, Config, ConfigValue};
use crate::source::{Refreshed, Source, SourceContext, SourceId};
use crate::util::font::{FontSpec, ResolvedFont};
use crate::util::store::ObjectId;
use anyhow::Context;
use chrono::{DateTime, Utc};
use indicatif::{MultiProgress, ProgressBar};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::sync::OnceCell as AsyncOnceCell;

/// Google Fonts source.
#[derive(Debug)]
pub struct GoogleFonts {
    pub config: GoogleFontsConfig,
    pub data: GoogleFontsData,
    index: AsyncOnceCell<Index>,
}
impl GoogleFonts {
    pub fn load(
        cli: &CliContext,
        main_config: &Config,
    ) -> anyhow::Result<Self> {
        let config = GoogleFontsConfig::load(cli, main_config)?;

        let data = if config.data_file.resolved.exists() {
            let data = std::fs::read_to_string(&config.data_file.resolved)?;
            let data: GoogleFontsData = toml::de::from_str(&data)?;
            data
        } else {
            // We don't write until we're doing clean-up
            GoogleFontsData::default()
        };

        Ok(Self {
            config,
            data,
            index: AsyncOnceCell::new(),
        })
    }

    pub async fn write_data(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.config.data_file.resolved.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let value = toml::ser::to_string(&self.data)?;
        tokio::fs::write(&self.config.data_file.resolved, value).await?;
        Ok(())
    }
}

/// Configuration for [`GoogleFonts`].
#[derive(Debug)]
pub struct GoogleFontsConfig {
    /// URL to a fresh index for this source.
    ///
    /// Sources:
    /// - Environment variable: `FONTPM_GOOGLE_FRESH_INDEX_URL`
    /// - `fontpm.toml` key: `google.fresh_index_url`
    /// - Default: A URL to a known-fresh index URL.
    pub fresh_index_url: ConfigValue<Arc<str>>,
    /// Path to the index.
    ///
    /// This file contains a single hash.
    ///
    /// Sources:
    /// - Default: `{store_dir}/google.toml`
    pub data_file: ConfigValue<Arc<Path>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GoogleFontsData {
    pub current_index: Option<CurrentIndex>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentIndex {
    pub object: Arc<ObjectId>,
    pub commit: String,
}

pub const DEFAULT_FRESH_INDEX_URL: &str =
    "https://raw.githubusercontent.com/fontpm/data/{ref}/google-fonts.json";
// TODO: Remove dependency on GitHub API
//       This could be done by including a timestamp in the index; if the
//       timestamp is newer, the index is too.
pub const DEFAULT_COMMIT_DATA_URL: &str =
    "https://api.github.com/repos/fontpm/data/branches/data";

impl GoogleFontsConfig {
    pub fn load(cli: &CliContext, config: &Config) -> anyhow::Result<Self> {
        let fresh_index_url = config!(
            env("FONTPM_GOOGLE_FRESH_INDEX_URL", then: |a| Ok(util::os_to_string(a))),
            toml(config.fontpm_toml, "google.fresh_index_url"),
            builtin(Ok(DEFAULT_FRESH_INDEX_URL.into()))
        )
            .fail(cli, "Google Fonts index URL")?
            .map(util::to_arc_str);
        let data_file = config!(
            toml(config.fontpm_toml, "google.data_file"),
            builtin(Ok(config.fontpm.store_dir.resolved.join("google.toml")))
        )
        .fail(cli, "Google Fonts data file")?
        .map(util::to_arc_path);

        Ok(Self {
            fresh_index_url,
            data_file,
        })
    }
}

#[async_trait]
impl Source for GoogleFonts {
    fn id(&self) -> &SourceId {
        &SourceId::GoogleFonts
    }

    async fn refresh_index(
        &mut self,
        context: &Arc<SourceContext>,
        pb: &ProgressBar,
    ) -> anyhow::Result<Refreshed> {
        pb.set_message("Checking for updates");
        // 1. Check if there's a new commit
        let response = context
            .http
            .get(Url::parse(DEFAULT_COMMIT_DATA_URL)?)
            .send()
            .await?
            .error_for_status()?;
        let commit_data = response.json::<GithubBranchData>().await?;

        let is_new = match &self.data.current_index {
            Some(index) => {
                // If the object doesn't exist in the index we force a refresh
                !context.store.object_exists(&index.object)
                    || index.commit != commit_data.commit.sha
            }
            None => true,
        };

        if !is_new {
            return Ok(Refreshed::AlreadyUpToDate);
        }

        let response = context
            .http
            .get(
                self.config
                    .fresh_index_url
                    .resolved
                    .replace("{ref}", &commit_data.commit.sha),
            )
            .send()
            .await?;
        let response = response.error_for_status()?;
        let tmp = context
            .store
            .download_to_tmp(
                response,
                "google-fonts-index",
                context.cli.modify_files,
                false,
                |current, total| {
                    if let Some(len) = total {
                        pb.set_length(len as u64)
                    }
                    pb.set_position(current as u64);
                },
            )
            .await?;

        let object = if let Some(object) = context.store.index_object(tmp.hash)
        {
            if context.cli.modify_files {
                tmp.move_to(
                    &object.path.to_path(&context.store.base_path),
                    &context.cli,
                )
                .await?;
            } else {
                tmp.discard(&context.cli).await;
            }
            object
        } else if let Some(object) = context.store.get_object(&tmp.hash) {
            let obj_path = object.path.to_path(&context.store.base_path);
            if !obj_path.exists() {
                tmp.move_to(&obj_path, &context.cli).await?;
            } else {
                tmp.discard(&context.cli).await;
            }
            object
        } else {
            anyhow::bail!("could not index Google Fonts index: object could not be indexed but does not exist");
        };

        let _ = writeln!(
            context.cli.debug(),
            "Google Fonts index updated to commit {}, object {}",
            commit_data.commit.sha,
            object.hash
        );

        self.data.current_index = Some(CurrentIndex {
            object: object.hash.clone(),
            commit: commit_data.commit.sha,
        });

        if context.cli.modify_files {
            pb.set_message("Writing data");
            self.write_data().await?;
        } else {
            let _ = writeln!(
                context.cli.warn_v(),
                "Google Fonts data would be written but will not be"
            );
        }

        Ok(Refreshed::Fresh)
    }

    async fn resolve_font(
        &self,
        context: &Arc<SourceContext>,
        font: &FontSpec,
    ) -> anyhow::Result<Vec<ResolvedFont>> {
        let Some(current_index) = &self.data.current_index else {
            anyhow::bail!("no current index")
        };
        let index = self.index.get_or_try_init(|| async {
            let Some(object) = context.store.get_object(&current_index.object) else {
                anyhow::bail!("current index object {} does not exist, please refresh", &current_index.object)
            };

            let path = context.store.resolve_object_path(&object.path);
            let data = tokio::fs::read(&path).await.context("reading index")?;
            let index: Index = serde_json::from_slice(&data).context("parsing index")?;
            Ok(index)
        }).await?;
        if let Some(family) = index.families.get(&font.id) {
            Ok(vec![ResolvedFont {
                id: family.id.to_string(),
                source: self.id().clone(),
                version: family.version.to_string(),
                timestamp: family.last_modified,
            }])
        } else {
            Ok(vec![])
        }
    }
}

#[derive(Deserialize)]
struct GithubCommitData {
    pub sha: String,
}
#[derive(Deserialize)]
struct GithubBranchData {
    pub commit: GithubCommitData,
}
#[derive(Clone, Debug, Deserialize)]
struct Index {
    families: HashMap<String, FontDescription>,
    /// Map of tags to a list of families that have that tag
    tags: HashMap<String, Vec<String>>,
}
#[derive(Clone, Debug, Deserialize)]
struct FontDescription {
    pub id: String,
    pub display_name: String,
    pub version: i32,
    pub tags: Vec<String>,
    #[serde(alias = "lastModified", with = "chrono::serde::ts_seconds")]
    pub last_modified: DateTime<Utc>,
    pub files: HashMap<String, String>,
    pub variants: Vec<String>,
}
