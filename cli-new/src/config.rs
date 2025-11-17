//! Configuration utilities.
//!
//! # Config directory (`config_dir`)
//!
//! Default location: `{dirs::preference_dir()}/fontpm`
//!
//! # Main config file (`config_file`)
//!
//! Default location: `{config_dir}/fontpm.toml`

use std::path::Path;
use std::sync::Arc;

pub struct ConfigPaths {
    /// Path to the configuration directory.
    ///
    /// Can be overridden by using the `FONTPM_CONFIG_DIR` environment variable.
    ///
    /// Example: `/home/Alice/.config/fontpm`
    pub config_dir: Arc<Path>,
    /// Path to the main configuration file, `fontpm.toml`.
    ///
    /// Can be overridden by using the `FONTPM_CONFIG_FILE` environment
    /// variable.
    ///
    /// Example: `/home/Alice/.config/fontpm/fontpm.toml`.
    pub fontpm_config_file: Arc<Path>,
}

impl ConfigPaths {
    pub fn load() -> Self {
        fn env_or_default<T>(
            key: impl AsRef<std::ffi::OsStr>,
            default: impl FnOnce() -> T,
        ) -> T
        where
            T: From<std::ffi::OsString>,
        {
            if let Some(value) = std::env::var_os(key) {
                T::from(value)
            } else {
                default()
            }
        }
        // NOTE(tecc): `cargo fmt` gives this formatting, even though it's ugly
        let config_dir: Arc<Path> = env_or_default("FONTPM_CONFIG_DIR", || {
            dirs::preference_dir()
                .expect("preference_dir required")
                .join("fontpm")
        })
        .into_boxed_path()
        .into();
        let fontpm_config_file: Arc<Path> =
            env_or_default("FONTPM_CONFIG_FILE", || {
                config_dir.join("fontpm.toml")
            })
            .into_boxed_path()
            .into();
        Self {
            config_dir,
            fontpm_config_file,
        }
    }
}
