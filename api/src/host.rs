use std::path::PathBuf;
use semver::Version;
pub use toml;

pub trait FpmHost: Sync {
    fn global_cache_dir(&self) -> PathBuf;
    fn cache_dir_for(&self, id: &str) -> PathBuf;
    fn font_install_dir(&self) -> PathBuf;
    fn config(&self, id: String) -> Option<&toml::Value>;
    fn version(&self) -> Version;
    fn user_agent(&self) -> String;
}

#[derive(Copy, Clone)]
pub struct EmptyFpmHost;
impl EmptyFpmHost {
    pub const EMPTY_HOST: EmptyFpmHost = EmptyFpmHost::new();
    pub const fn new() -> Self {
        return EmptyFpmHost {};
    }
}

impl FpmHost for EmptyFpmHost {
    fn global_cache_dir(&self) -> PathBuf {
        unimplemented!()
    }

    fn cache_dir_for(&self, _: &str) -> PathBuf {
        unimplemented!()
    }

    fn font_install_dir(&self) -> PathBuf {
        unimplemented!()
    }

    fn config(&self, _: String) -> Option<&toml::Value> {
        unimplemented!()
    }

    fn version(&self) -> Version {
        unimplemented!()
    }
    fn user_agent(&self) -> String {
        format!("FontPM-Host/{}", self.version())
    }
}