use std::collections::HashMap;
use std::path::{Path, PathBuf};
use path_calculate::Calculate;
use fontpm_api::font::{DefinedFontVariantSpec, FontDescription};

#[derive(Clone)]
pub struct Generate {
    target_file: PathBuf,
    fonts: HashMap<(FontDescription, DefinedFontVariantSpec), PathBuf>
}

#[derive(thiserror::Error, Debug)]
pub enum GenerateError {
    #[error("relative_path error: {0}")]
    PathError(std::io::Error)
}

impl Generate {
    pub fn from_font(target_file: impl AsRef<Path>, desc: impl AsRef<FontDescription>, map: HashMap<DefinedFontVariantSpec, impl AsRef<Path>>) -> Self {
        let target_file = target_file.as_ref().to_path_buf();
        let desc = desc.as_ref();
        let mut fonts = HashMap::new();
        for (spec, path) in map {
            fonts.insert((desc.clone(), spec), path.as_ref().to_path_buf());
        }
        Self {
            target_file,
            fonts
        }
    }
    pub fn generate_css(&self) -> Result<String, GenerateError> {
        let mut stylesheet = String::new();

        let target_parent = self.target_file.parent().unwrap();
        for ((desc, _spec), path) in &self.fonts {
            let relation = path.related_to(&target_parent).map_err(GenerateError::PathError)?.to_path_buf();

            let rule = format!(r#"
@font-face {{
/*{desc:?}*/
    font-family: "{}";
    src: url("{}");
}}
            "#, desc.name, relation.display());
            stylesheet.push_str(rule.as_str());
        }

        Ok(stylesheet)
    }
}