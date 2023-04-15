use fontpm_api::{Error, FpmHost, Source};
use fontpm_source_google_fonts::GoogleFontsSource;
use crate::config::FpmConfig;

/// Create a single source instance.
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
pub fn create_source<'host>(source: String, _host: Option<&'host dyn FpmHost>) -> Option<Box<dyn Source<'host> + 'host>> {
    macro_rules! source {
        ($source:expr) => {
            Some({
                let mut source = $source;
                if let Some(host) = _host {
                    source.set_host(host);
                }
                Box::new(source)
            })
        }
    }
    match source.as_str() {
        #[cfg(feature = "google-fonts")]
        GoogleFontsSource::ID => source!(GoogleFontsSource::new()),
        _ => None
    }
}

pub fn create_sources<'host>(host: Option<&'host dyn FpmHost>, only: Option<Vec<&String>>) -> fontpm_api::Result<Vec<Box<dyn Source<'host> + 'host>>> {
    let config = FpmConfig::load()?.clone();

    let only = if let Some(v) = only { v.into_iter().collect() } else { Vec::new() };

    return Ok(config.enabled_sources.into_iter()
        .filter_map(|v| {
            if only.len() > 0 {
                if !only.contains(&&v) {
                    return None
                }
            }
            create_source(v, host)
        })
        .collect()
    );
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FontSpec {
    pub source: Option<String>,
    pub font_id: String
}

impl FontSpec {
    pub fn parse<S>(v: S) -> Result<FontSpec, Error> where S: ToString {
        let v = v.to_string();
        if v.is_empty() {
            return Err(Error::Generic("Fontspec must not be an empty string".into()))
        }

        let mut source = None;
        let mut current = String::new();
        for c in v.chars() {
            if c == ':' {
                if source.is_some() {
                    return Err(Error::Generic("Character ':' is illegal in font ID".into()))
                }
                source = Some(current.clone());
                current.clear();
            }
            current.push(c)
        }

        Ok(FontSpec {
            source,
            font_id: current
        })
    }
}