use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct FontDescription {
    id: String,
    display_name: String,
    version: i32,
    tags: Vec<String>,
    #[serde(alias = "lastModified")]
    last_modified: u64,
    files: HashMap<String, String>,
    variants: Vec<String>
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Data {
    families: HashMap<String, FontDescription>,
    tags: HashMap<String, Vec<String>>
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