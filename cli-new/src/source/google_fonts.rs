//! Google Fonts source.

use crate::cli::CliContext;
use crate::config::ConfigValue;
use crate::source::{Refreshed, Source, SourceId};
use std::sync::Arc;

/// Google Fonts source.
pub struct GoogleFonts {}

/// Configuration for [`GoogleFonts`].
pub struct GoogleFontsConfig {
    /// URL to a fresh index for this source.
    ///
    /// Sources:
    /// - Environment variable: `FONTPM_GOOGLE_FRESH_INDEX_URL`
    /// - `fontpm.toml` key: `google.fresh_index_url`
    /// - Default: A URL to a known-fresh index URL.
    pub fresh_index_url: ConfigValue<Arc<str>>,
}

#[async_trait]
impl Source for GoogleFonts {
    fn id(&self) -> &SourceId {
        &SourceId::GoogleFonts
    }

    async fn refresh_index(
        context: Arc<CliContext>,
    ) -> anyhow::Result<Refreshed> {
        todo!()
    }
}
