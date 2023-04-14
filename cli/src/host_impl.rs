use std::path::PathBuf;
use semver::Version;
use toml::value::Table;
use fontpm_api::FpmHost;
use crate::config::{EntireConfig, FpmConfig};

pub struct FpmHostImpl {
    cache_dir: PathBuf,
    font_install_dir: PathBuf,
    config: EntireConfig,
}

impl FpmHostImpl {
    pub fn create(font_install_dir: Option<PathBuf>) -> fontpm_api::Result<FpmHostImpl> {
        let cfg = EntireConfig::load()?;
        let fontpm = cfg.fontpm();
        Ok(FpmHostImpl {
            cache_dir: fontpm.cache_dir(),
            font_install_dir: font_install_dir.unwrap_or(fontpm.font_install_dir()),
            config: cfg,
        })
    }
}

impl FpmHost for FpmHostImpl {
    fn global_cache_dir(&self) -> PathBuf {
        self.cache_dir.clone()
    }

    fn cache_dir_for(&self, id: &str) -> PathBuf {
        let mut clone = self.cache_dir.clone();
        clone.push(id);
        clone
    }

    fn font_install_dir(&self) -> PathBuf {
        self.font_install_dir.clone()
    }

    fn config(&self, id: String) -> Option<&toml::Value> {
        return self.config.get_source_config(id)
    }

    fn version(&self) -> Version {
        crate::VERSION.clone()
    }

    fn user_agent(&self) -> String {
        format!("FontPM/{}", self.version())
    }
}