//! Google Fonts source.

use crate::cli::CliContext;
use crate::config::{config, util, util::os_to_url, Config, ConfigValue};
use crate::source::{Refreshed, Source, SourceContext, SourceId};
use crate::util::store::ObjectId;
use reqwest::Url;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Google Fonts source.
pub struct GoogleFonts {
    pub config: GoogleFontsConfig,
    pub data: GoogleFontsData,
}

/// Configuration for [`GoogleFonts`].
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
    /// - Default: ``
    pub data_file: ConfigValue<Arc<Path>>,
}

pub struct GoogleFontsData {
    pub current_index: Option<CurrentIndex>,
}

pub struct CurrentIndex {
    pub object: Arc<ObjectId>,
    pub commit: String,
}

pub const DEFAULT_FRESH_INDEX_URL: &str =
    "https://raw.githubusercontent.com/fontpm/data/{tag}/google-fonts.json";
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
            builtin(Ok(PathBuf::from("google.toml")))
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
        context: Arc<SourceContext>,
    ) -> anyhow::Result<Refreshed> {
        // 1. Check if there's a new commit
        let response = context
            .http
            .get(Url::parse(DEFAULT_FRESH_INDEX_URL)?)
            .send()
            .await?
            .error_for_status()?;
        let commit_data = response.json::<GithubBranchData>().await?;

        let is_new = match &self.data.current_index {
            Some(index) => index.commit != commit_data.commit.sha,
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
                true,
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

        self.data.current_index = Some(CurrentIndex {
            object: object.hash.clone(),
            commit: commit_data.commit.sha,
        });

        Ok(Refreshed::Fresh)
    }
}

#[derive(Deserialize)]
pub struct GithubCommitData {
    pub sha: String,
}
#[derive(Deserialize)]
pub struct GithubBranchData {
    pub commit: GithubCommitData,
}
