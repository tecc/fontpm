use fontpm_api::Source;

pub fn sources() -> Vec<String> {
    let mut vec: Vec<String> = vec!();

    // Now here comes the fun part!
    #[cfg(feature = "google-fonts")]
    vec.push(fontpm_source_google_fonts::GoogleFontsSource::ID.into());

    vec
}