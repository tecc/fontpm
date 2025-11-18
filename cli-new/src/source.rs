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

use crate::cli::CliContext;
use std::sync::Arc;

#[cfg(feature = "source-google-fonts")]
mod google_fonts;

#[async_trait]
pub trait Source {
    /// Refresh the local index.
    async fn refresh_index(
        context: Arc<CliContext>,
    ) -> anyhow::Result<Refreshed>;
}

/// More information about the result of refreshing the index.
pub enum Refreshed {
    /// The local index for the source was already up-to-date.
    AlreadyUpToDate,
    /// The local index has been replaced with a fresher index
    Fresh,
}

macro_rules! sources {
    (
        $(
        #[$feature_meta:meta]
        $source_ty:ty {
             field: $source_name:ident,
             id: $source_id:ident,
             str: [$source_str_first:literal $(, $source_str:literal)*]
        }
        )?
    ) => {
        pub struct Sources {
            $(
            #[$feature_meta]
            $source_name: $source_ty
            )*
        }
        impl Sources {
            // TODO: Utilities for constructing sources
        }
        #[derive(Copy, Clone, serde::Serialize, serde::Deserialize)]
        pub enum SourceId {
            $(
            #[serde(rename = $source_str_first)]
            $( #[serde(alias = $source_str)] )*
            $source_id
            )*
        }
        impl SourceId {
            pub fn is_enabled(self) -> bool {
                match self {
                    $(
                    Self::$source_id => {
                        let result = false;
                        #[$feature_meta]
                        let result = true;
                        result
                    }
                    )*
                }
            }
        }
    };
}

sources!(
    #[cfg(feature = "source-google-fonts")]
    google_fonts::GoogleFonts {
        field: google_fonts,
        id: GoogleFonts,
        str: ["Google Fonts", "google"]
    }
);
