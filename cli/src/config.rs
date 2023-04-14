use std::{collections::HashMap, io, path::PathBuf};
use std::fs::{File};
use std::io::{Read};
use serde::{Deserialize, Serialize};
use toml::Value;
use fontpm_api::{Result as FResult, Error};
use fontpm_api::util::create_parent;
use crate::build_config;

#[derive(Default, Deserialize, Serialize, Clone, PartialEq)]
pub struct EntireConfig {
    fontpm: FpmConfig,
    #[serde(skip_serializing_if = "HashMap::is_empty", default = "HashMap::new")]
    sources: HashMap<String, Value>
}

impl EntireConfig {
    pub fn get_source_config(&self, id: String) -> Option<&Value> {
        return self.sources.get(&id);
    }
}

static mut CONFIG: Option<EntireConfig> = None;

impl EntireConfig {
    pub fn fontpm(&self) -> FpmConfig {
        return self.fontpm.clone();
    }

    pub fn config_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap();
        path.push("fontpm");

        path
    }
    pub fn config_file() -> PathBuf {
        let mut path = Self::config_dir();
        path.push("config.toml");

        path
    }

    pub fn write_to<Output>(&self, output: &mut Output) -> FResult<()> where Output: io::Write {
        let value = toml::ser::to_string(self)
            .map_err(|v| Error::Serialisation(v.to_string()))?;

        output.write_all(value.as_bytes())?;

        Ok(())
    }

    pub fn force_load() -> FResult<EntireConfig> {
        let config_path = Self::config_file();

        let cfg = if !config_path.exists() {
            create_parent(&config_path)?;

            let default_config = EntireConfig::default();

            let mut file = File::create(config_path)?;
            default_config.write_to(&mut file)?;

            default_config
        } else {
            let mut file = File::open(config_path)?;
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;

            toml::from_str::<EntireConfig>(buffer.as_str())
                .map_err(|v| Error::Deserialisation(v.to_string()))?
        };

        unsafe {
            CONFIG = Some(cfg.clone());
        }

        Ok(cfg)
    }
    pub fn load() -> fontpm_api::Result<EntireConfig> {
        unsafe {
            if CONFIG == None {
                Self::force_load()
            } else {
                Ok(CONFIG.clone().unwrap())
            }
        }
    }
}


#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct FpmConfig {
    pub enabled_sources: Vec<String>,
    pub cache_dir: Option<PathBuf>,
    pub font_install_dir: Option<PathBuf>
}

impl FpmConfig {
    pub fn load() -> fontpm_api::Result<FpmConfig> {
        let whole = EntireConfig::load()?;
        Ok(whole.fontpm)
    }

    pub fn cache_dir(&self) -> PathBuf {
        return self.cache_dir.clone().unwrap_or_else(|| {
            let mut cache_dir = dirs::cache_dir().expect("Cache dir required");
            cache_dir.push("fontpm");
            cache_dir
        })
    }

    pub fn font_install_dir(&self) -> PathBuf {
        return self.font_install_dir.clone().unwrap_or_else(|| {
            let mut cache_dir = dirs::font_dir().expect("Font dir required");
            cache_dir.push("fontpm");
            cache_dir
        })
    }
}

impl Default for FpmConfig {
    fn default() -> Self {
        return FpmConfig {
            enabled_sources: build_config::sources(),
            cache_dir: None,
            font_install_dir: None
        }
    }
}