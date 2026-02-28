//! Google Fonts source.

use crate::cli::CliContext;
use crate::config::{config, util, Config, ConfigValue};
use crate::source::{
    HasSubcommand, Refreshed, Source, SourceContext, SourceId, SourceSubcommand,
};
use crate::util::font::{
    Download, FontFile, FontFileKind, FontIdentifier, FontReference, FontSpec,
    ResolvedFont,
};
use crate::util::store::ObjectId;
use crate::util::{impl_serde_as_string, string_enum};
use anyhow::Context;
use chrono::{DateTime, Utc};
use clap::{Args, Command, Parser, Subcommand};
use indicatif::ProgressBar;
use relative_path::{RelativePath, RelativePathBuf};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;
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

#[derive(Args)]
pub struct GoogleFontsArgs {
    #[command(subcommand)]
    command: SourceCommand,
}
#[derive(Subcommand)]
enum SourceCommand {
    Index(IndexArgs),
}

/// Index the Google Fonts git repository.
#[derive(Args)]
struct IndexArgs {
    repository: PathBuf,
}

impl HasSubcommand for GoogleFonts {
    type Args = GoogleFontsArgs;

    fn execute(
        &mut self,
        cli: CliContext,
        config: Config,
        args: Self::Args,
    ) -> ExitCode {
        ExitCode::SUCCESS
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
        let expected_length = response.content_length();
        if let Some(total) = expected_length {
            pb.set_length(total);
        }
        let tmp = context
            .store
            .download_to_tmp(
                response.bytes_stream(),
                expected_length,
                "google-fonts-index",
                context.cli.modify_files,
                false,
                |current| {
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
                reference: FontReference {
                    identifier: FontIdentifier {
                        id: family.id.to_string(),
                        source: self.id().clone(),
                    },
                    version: family.version.to_string(),
                    timestamp: family.last_modified,
                },
                files: family
                    .variants
                    .iter()
                    .filter_map(|variant| {
                        family.files.get(variant).map(|url_no_proto| {
                            let download_url = Url::parse(&format!(
                                "https://{}",
                                url_no_proto
                            ))
                            .unwrap();
                            let path = RelativePath::new(download_url.path());
                            let (dot, ext) = if let Some(ext) = path.extension()
                            {
                                (".", ext)
                            } else {
                                ("", "")
                            };
                            FontFile {
                                name: format!(
                                    "{}-{}{}{}",
                                    family.id, variant, dot, ext
                                )
                                .into(),
                                kind: FontFileKind::Family {
                                    axes: vec![],
                                    weight: Some(variant.weight.as_u32()),
                                    italic: variant.italic,
                                },
                                download: Download::Url(download_url),
                            }
                        })
                    })
                    .collect(),
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
    pub files: HashMap<Variant, String>,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Variant {
    weight: Weight,
    italic: bool,
}
impl_serde_as_string!(impl for Variant);
impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.weight, self.italic) {
            (Weight::Regular, false) => write!(f, "regular"),
            (Weight::Regular, true) => write!(f, "italic"),
            (weight, false) => write!(f, "{}", weight),
            (weight, true) => write!(f, "{}italic", weight),
        }
    }
}
impl FromStr for Variant {
    type Err = crate::util::NoSuchVariant;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "regular" => Ok(Self {
                weight: Weight::Regular,
                italic: false,
            }),
            "italic" => Ok(Self {
                weight: Weight::Regular,
                italic: true,
            }),
            s => {
                let (s, italic) = if let Some(weight) = s.strip_suffix("italic")
                {
                    (weight, true)
                } else {
                    (s, false)
                };
                Ok(Self {
                    weight: s.parse()?,
                    italic,
                })
            }
        }
    }
}
string_enum!(
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Weight {
        _100 = "100",
        _200 = "200",
        _300 = "300",
        Regular = "regular",
        _500 = "500",
        _600 = "600",
        _700 = "700",
        _800 = "800",
        _900 = "900",
    }
);
impl Weight {
    fn as_u32(self) -> u32 {
        match self {
            Self::_100 => 100,
            Self::_200 => 200,
            Self::_300 => 300,
            Self::Regular => 400,
            Self::_500 => 500,
            Self::_600 => 600,
            Self::_700 => 700,
            Self::_800 => 800,
            Self::_900 => 900,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_variants() {
        for (input, weight, italic) in [
            ("100", Weight::_100, false),
            ("100italic", Weight::_100, true),
            ("200", Weight::_200, false),
            ("200italic", Weight::_200, true),
            ("300", Weight::_300, false),
            ("300italic", Weight::_300, true),
            ("regular", Weight::Regular, false),
            ("italic", Weight::Regular, true),
            ("500", Weight::_500, false),
            ("500italic", Weight::_500, true),
            ("600", Weight::_600, false),
            ("600italic", Weight::_600, true),
            ("700", Weight::_700, false),
            ("700italic", Weight::_700, true),
            ("800", Weight::_800, false),
            ("800italic", Weight::_800, true),
            ("900", Weight::_900, false),
            ("900italic", Weight::_900, true),
        ] {
            let parsed_variant = Variant::from_str(input).unwrap();
            assert_eq!(parsed_variant.weight, weight);
            assert_eq!(parsed_variant.italic, italic);
            assert_eq!(input, parsed_variant.to_string());
        }
    }
}
