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

use crate::{cli::CliContext, config::Config};
use std::sync::Arc;

#[cfg(feature = "source-google-fonts")]
mod google_fonts;

#[async_trait]
pub trait Source {
    /// Get the ID for this source.
    fn id(&self) -> &SourceId;
    /// Refresh the local index.
    async fn refresh_index(
        context: Arc<CliContext>,
    ) -> anyhow::Result<Refreshed>;
}

/// More information about the result of refreshing the index.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Refreshed {
    /// The local index for the source was already up-to-date.
    AlreadyUpToDate,
    /// The local index has been replaced with a fresher index
    Fresh,
}

pub struct Sources {
    #[cfg(feature = "source-google-fonts")]
    google_fonts: Option<google_fonts::GoogleFonts>,
}
impl Sources {
    pub fn create_enabled(cli: &CliContext, config: &Config) {
        let sources = Self { google_fonts: None };
        for id in config.fontpm.enabled_sources.resolved.as_ref() {
            // NOTE(tecc): Keep feature-gating on the arm itself and not inside
            //             it; there's no point in having extra indentation.
            match id {
                #[cfg(feature = "source-google-fonts")]
                SourceId::GoogleFonts => {
                    if sources.google_fonts.is_some() {
                        let _ = writeln!(
                            cli.warn(),
                            "The Google Fonts source is alread"
                        );
                        continue;
                    }
                    todo!("Create Google Fonts source")
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
    }
);

source_id!(
    SourceId:
    GoogleFonts "google-fonts" ["google"]
);
