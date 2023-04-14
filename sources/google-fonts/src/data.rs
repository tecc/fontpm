use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use fontpm_api::Error;
use fontpm_api::font::{DefinedFontInstallSpec, DefinedFontVariantSpec, DefinedFontWeight};

#[derive(Deserialize, Serialize, Clone)]
pub struct FontDescription {
    pub id: String,
    pub display_name: String,
    pub version: i32,
    pub tags: Vec<String>,
    #[serde(alias = "lastModified")]
    pub last_modified: u64,
    pub files: HashMap<String, String>,
    pub variants: Vec<String>
}

impl TryFrom<FontDescription> for DefinedFontInstallSpec {
    type Error = Error;

    fn try_from(value: FontDescription) -> Result<Self, Self::Error> {
        let variants: Result<Vec<DefinedFontVariantSpec>, Error> =
            value.variants.into_iter().map(|v| description::string_to_variant(v)).collect();
        let variants = variants?;

        Ok(DefinedFontInstallSpec::new(value.id, variants))
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Data {
    families: HashMap<String, FontDescription>,
    tags: HashMap<String, Vec<String>>
}

const REGULAR_WEIGHT: DefinedFontWeight =  DefinedFontWeight::Fixed(400);

pub mod description {
    use std::mem::swap;
    use std::str::FromStr;
    use fontpm_api::Error;
    use fontpm_api::font::{DefinedFontStyle, DefinedFontVariantSpec, DefinedFontWeight};
    use crate::data::REGULAR_WEIGHT;

    pub fn string_to_variant<S>(str: S) -> Result<DefinedFontVariantSpec, Error> where S: AsRef<str> {
        let str = str.as_ref();
        match str {
            "regular" => return Ok(DefinedFontVariantSpec { weight: REGULAR_WEIGHT, style: DefinedFontStyle::Regular }),
            "italic" => return Ok(DefinedFontVariantSpec { weight: REGULAR_WEIGHT, style: DefinedFontStyle::Italic }),
            _ => {}
        }
        let mut weight: &str = "";
        let mut type_str: &str = "";
        let mut state = 0;
        let mut pi = 0;
        for (i, c) in str.char_indices() {
            match state {
                0 => {
                    if !c.is_numeric() {
                        pi = i;
                        weight = &str[0..pi];
                        state = 1;
                        continue;
                    }
                },
                1 => {
                }
                _ => panic!("logic error")
            }
        }
        type_str = &str[pi..str.len()];
        if weight == "" {
            swap(&mut weight, &mut type_str);
        }

        Ok(DefinedFontVariantSpec {
            weight: DefinedFontWeight::Fixed(u32::from_str(weight).map_err(|e| Error::Deserialisation(e.to_string()))?),
            style: DefinedFontStyle::from_str(type_str)?,
        })
    }
    pub fn variant_to_string(variant: &DefinedFontVariantSpec) -> String {
        if variant.weight == REGULAR_WEIGHT {
            return match variant.style {
                DefinedFontStyle::Regular => "regular".to_string(),
                DefinedFontStyle::Italic => "italic".to_string()
            }
        }
        let weight = match variant.weight {
            DefinedFontWeight::Fixed(weight) => weight.to_string(),
            DefinedFontWeight::Variable => "wght".to_string() // isn't actually used so...
        };
        let style = match variant.style {
            DefinedFontStyle::Regular => "",
            DefinedFontStyle::Italic => "italic"
        };
        weight + style
    }
}

impl Data {
    pub fn get_family(&self, id: &String) -> Option<FontDescription> {
        self.families.get(id).map(|v| v.clone())
    }
    pub fn get_all_families(&self) -> Vec<String> {
        return Vec::from_iter(self.families.keys().map(Clone::clone));
    }
    pub fn get_all_families_with_tag(&self, tag: &String) -> Vec<String> {
        return self.tags.get(tag).map_or(Vec::new(), Clone::clone);
    }
    pub fn search_by_tags(&self, tags: &Vec<String>) -> Vec<String> {
        if tags.len() < 1 {
            return self.get_all_families()
        } else if tags.len() < 2 {
            return self.get_all_families_with_tag(tags.first().unwrap())
        }

        let mut tags = tags.clone();

        let mut smallest_tag: String = String::new();
        let mut smallest_tag_vec: Vec<String> = Vec::new();
        let mut smallest_tag_vec_len: usize = usize::MAX;
        for tag in &tags {
            let other = self.get_all_families_with_tag(tag);
            if other.len() < smallest_tag_vec_len {
                smallest_tag = tag.clone();
                smallest_tag_vec_len = other.len();
                smallest_tag_vec = other;
            }
        }

        let mut set: HashSet<String> = HashSet::from_iter(smallest_tag_vec.into_iter());
        for tag in &tags {
            let tag = tag.clone();
            if tag == smallest_tag {
                continue;
            }
            set = set.into_iter()
                .filter(|v| self.get_family(v).unwrap().tags.contains(&tag))
                .collect()
        }

        Vec::from_iter(set.into_iter())
    }
}