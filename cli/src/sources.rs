use fontpm_api::{FpmHost, Source};
use fontpm_source_google_fonts::GoogleFontsSource;
use crate::config::FpmConfig;

///
///
/// # Arguments
///
/// * `source`: The ID of the source to create
///
/// returns: `Some` if the ID refers to an available source, otherwise `None`.
///
/// # Examples
///
/// ```
/// // create a source using a valid ID (google-fonts)
/// // note: this example only works if the Google Fonts feature is enabled
/// let id = String::from("google-fonts");
/// let source = create_source(&id);
/// assert!(source.is_some());
/// ```
/// ```
/// // attempt to create a source using an invalid ID
/// let id = String::from("some-invalid-id");
/// let source = create_source(&id);
/// assert!(source.is_none());
/// ```
pub fn create_source<'a>(source: String, host: Option<&'a dyn FpmHost>) -> Option<impl Source<'a>> {
    macro_rules! source {
        ($source:expr) => {
            Some({
                let mut source = $source;
                if let Some(host) = host {
                    source.set_host(host);
                }
                source
            })
        }
    }
    match source.as_str() {
        #[cfg(feature = "google-fonts")]
        GoogleFontsSource::ID => source!(GoogleFontsSource::new()),
        _ => None
    }
}

pub fn create_enabled_sources<'a>(host: Option<&'a dyn FpmHost>) -> fontpm_api::Result<Vec<impl Source<'a>>> {
    let config = FpmConfig::load()?.clone();

    return Ok(config.enabled_sources.into_iter()
        .filter_map(|v| create_source(v, host))
        .collect()
    );
}