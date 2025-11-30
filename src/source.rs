//! Sources.
//!
//! # General operations for all sources
//!
//! ## Refreshing local index
//!
//! Refreshing the index of a source and keeping a cached version of it locally.
//!
//! ## Searching fonts
//!
//! It should be possible to search through the indexed fonts quickly.
//!
//! # Advanced operations
//!
//! ## Generating new indices
//!
//! Some sources require generating an index (e.g. Google Fonts). FontPM should
//! support doing so within the CLI itself, instead of relying on outside
//! programs to.

use crate::util::font::{FontSpec, ResolvedFont};
use crate::util::store::ObjectStore;
use crate::{cli::CliContext, config::Config};
use anyhow::Context;
use std::ffi::OsString;
use std::fmt;
use std::process::ExitCode;
use std::sync::Arc;

#[cfg(feature = "source-google-fonts")]
pub mod google_fonts;

#[async_trait]
pub trait Source: Send {
    /// Get the ID for this source.
    fn id(&self) -> &SourceId;

    fn create_progress_bar(
        &self,
        mpb: &indicatif::MultiProgress,
    ) -> indicatif::ProgressBar {
        let pb = indicatif::ProgressBar::new_spinner()
            .with_prefix(self.id().canonical_name().to_string());
        mpb.add(pb)
    }

    /// Refresh the local index.
    async fn refresh_index(
        &mut self,
        context: &Arc<SourceContext>,
        pb: &indicatif::ProgressBar,
    ) -> anyhow::Result<Refreshed>;

    /// Resolve a font spec to a list of matching fonts.
    ///
    /// # Return value
    ///
    /// If matching fonts were found, return a list of those matching fonts
    /// `Ok(matching_fonts)`.
    ///
    /// If no matching fonts were found, return `Ok(vec![])`.
    ///
    /// If an unexpected error occurs whilst attempting to resolve the font
    /// spec, return the `Err` variant.
    async fn resolve_font(
        &self,
        context: &Arc<SourceContext>,
        font: &FontSpec,
    ) -> anyhow::Result<Vec<ResolvedFont>>;

    fn execute_command(&mut self, command: SourceSubcommand) -> ExitCode;
}

pub struct SourceSubcommand {
    pub cli: CliContext,
    pub config: Config,
    pub args: Vec<OsString>,
}

pub type AnySource = Box<dyn Source>;
pub fn new_any_source<T>(source: T) -> AnySource
where
    T: Source + 'static,
{
    Box::new(source)
}

/// More information about the result of refreshing the index.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Refreshed {
    /// The local index for the source was already up-to-date.
    AlreadyUpToDate,
    /// The local index has been replaced with a fresher index
    Fresh,
}

#[derive(Clone)]
pub struct SourceContext {
    pub cli: Arc<CliContext>,
    pub store: Arc<ObjectStore>,
    pub http: reqwest::Client,
}
impl SourceContext {
    pub fn new(cli: Arc<CliContext>, config: &Config) -> anyhow::Result<Self> {
        let store = ObjectStore::load(&cli, &config)
            .context("could not load object store")?;

        Ok(Self {
            cli,
            store,
            http: reqwest::Client::builder()
                .user_agent(config.fontpm.http.user_agent.resolved.clone())
                .build()
                .context("could not create HTTP client")?,
        })
    }
}

#[derive(Debug)]
pub struct Sources {
    #[cfg(feature = "source-google-fonts")]
    google_fonts: Option<google_fonts::GoogleFonts>,
}
impl Sources {
    pub fn create_enabled(
        cli: &CliContext,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let mut sources = Self { google_fonts: None };
        for id in config.fontpm.enabled_sources.resolved.as_ref() {
            // NOTE(tecc): Keep feature-gating on the arm itself and not inside
            //             it; there's no point in having extra indentation.
            match id {
                #[cfg(feature = "source-google-fonts")]
                SourceId::GoogleFonts => {
                    if sources.google_fonts.is_some() {
                        let _ = writeln!(
                            cli.warn(),
                            "The Google Fonts source has already been registered"
                        );
                        continue;
                    }
                    let source = google_fonts::GoogleFonts::load(cli, config)?;
                    sources.google_fonts = Some(source);
                }
                #[cfg(not(feature = "source-google-fonts"))]
                SourceId::GoogleFonts => {
                    let _ = writeln!(cli.warn(), "The Google Fonts source was enabled, but this build of FontPM does not support it; ignoring");
                }
                SourceId::Other(other) => {
                    let _ = writeln!(
                        cli.warn(),
                        "An unrecognised source '{}' is enabled; ignoring",
                        other
                    );
                }
            }
        }
        Ok(sources)
    }

    pub fn create_specific(
        cli: &CliContext,
        config: &Config,
        id: &SourceId,
    ) -> Option<anyhow::Result<AnySource>> {
        match id {
            #[cfg(feature = "source-google-fonts")]
            SourceId::GoogleFonts => {
                let source = google_fonts::GoogleFonts::load(cli, config)
                    .map(new_any_source);

                Some(source)
            }
            #[cfg(not(feature = "source-google-fonts"))]
            SourceId::GoogleFonts => Some(Err(anyhow::anyhow!(
                "this build does not support Google Fonts source"
            ))),
            SourceId::Other(_other) => None,
        }
    }

    pub fn is_enabled(&self, id: &SourceId) -> bool {
        self.iter().any(|source| source.id() == id)
    }

    pub fn get<'a>(&'a self, id: &SourceId) -> Option<&'a dyn Source> {
        fn as_dyn_source(x: &impl Source) -> &dyn Source {
            x
        }
        match id {
            #[cfg(feature = "source-google-fonts")]
            SourceId::GoogleFonts => {
                self.google_fonts.as_ref().map(as_dyn_source)
            }
            _ => None,
        }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a dyn Source> {
        std::iter::once(self.google_fonts.as_ref().map(|a| a as &dyn Source))
            .flatten()
    }
    /// Get an iterator over mutable references to every source.
    pub fn iter_mut<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = &'a mut dyn Source> {
        std::iter::once(
            self.google_fonts.as_mut().map(|a| a as &mut dyn Source),
        )
        .flatten()
    }
}

pub static DEFAULT_ENABLED_SOURCES: &'static [SourceId] =
    &[SourceId::GoogleFonts];

/// A source ID.
///
/// This enum can represent a well-known source ID, as well as custom source
/// IDs should FontPM ever need to (e.g. for custom sources).
///
/// A source ID has a *canonical name* which is unique to the source.
///
/// A source ID may have one or more *aliases* which are alternative names that
/// can be used to refer to it.
///
/// Even if the implementation of a source is feature-gated, its ID is always
/// known.
///
/// Names are case-sensitive. As a rule, names should be lowercase.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(from = "&str")]
pub enum SourceId {
    /// Google Fonts.
    ///
    /// Canonical name: `google-fonts`
    /// Aliases: `google`
    GoogleFonts,
    Other(Arc<str>),
}

macro_rules! source_id (
    (
        $typename:ident:
        $(
        $variant:ident $canonical_name:literal [$( $alias:literal )* ]
        )*
    ) => {
        impl $typename {
            pub const KNOWN_NAMES: &'static [&'static str] = &[
                $(
                    $canonical_name
                    $(, $alias)*
                ),*
            ];

            pub fn try_reserved_name(name: &str) -> Option<Self> {
                match name {
                    $(
                        $canonical_name $(| $alias)* => Some(Self::$variant),

                    )*
                    _ => None
                }
            }

            pub fn canonical_name(&self) -> &str {
                match self {
                    $(
                    Self::$variant => $canonical_name,
                    )*
                    Self::Other(name) => name.as_ref()
                }
            }
        }
        impl From<Arc<str>> for $typename {
            fn from(value: Arc<str>) -> $typename {
                if let Some(x) = Self::try_reserved_name(value.as_ref()) {
                    x
                } else {
                    Self::Other(value)
                }
            }
        }
        impl<'a> From<&'a str> for $typename {
            fn from(value: &'a str) -> $typename {
                if let Some(x) = Self::try_reserved_name(value) {
                    x
                } else {
                    Self::Other(Arc::from(value))
                }
            }
        }
        impl std::str::FromStr for $typename {
            type Err = std::convert::Infallible;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self::from(s))
            }
        }
        /// SourceId is ordered by the ordering of their respective canonical
        /// names.
        impl Ord for $typename {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.canonical_name().cmp(other.canonical_name())
            }
        }
        impl PartialOrd for $typename {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(Self::cmp(self, other))
            }
        }
        impl Eq for $typename {}
        impl PartialEq for $typename {
            fn eq(&self, other: &Self) -> bool {
                self.canonical_name().eq(other.canonical_name())
            }
        }
        impl serde::Serialize for $typename {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer
            {
                serializer.serialize_str(self.canonical_name())
            }
        }
        impl fmt::Display for $typename {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(self.canonical_name())
            }
        }
    }
);

source_id!(
    SourceId:
    GoogleFonts "google-fonts" ["google"]
);
