//! Configuration utilities.
//!
//! # Config directory (`config_dir`)
//!
//! Default location: `{dirs::preference_dir()}/fontpm`
//!
//! # Main config file (`config_file`)
//!
//! Default location: `{config_dir}/fontpm.toml`

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ConfigPaths {
    /// Path to the configuration directory.
    ///
    /// Can be overridden by using the `FONTPM_CONFIG_DIR` environment variable.
    ///
    /// Example: `/home/Alice/.config/fontpm`
    pub config_dir: ConfigValue<Arc<Path>>,
    /// Path to the main configuration file, `fontpm.toml`.
    ///
    /// Can be overridden by using the `FONTPM_CONFIG_FILE` environment
    /// variable.
    ///
    /// Example: `/home/Alice/.config/fontpm/fontpm.toml`.
    pub fontpm_config_file: ConfigValue<Arc<Path>>,
}

impl ConfigPaths {
    pub fn load() -> Self {
        fn env_or_default<T>(
            key: impl AsRef<std::ffi::OsStr>,
            default: impl FnOnce() -> T,
        ) -> T
        where
            T: From<OsString>,
        {
            if let Some(value) = std::env::var_os(key) {
                T::from(value)
            } else {
                default()
            }
        }
        // NOTE(tecc): `cargo fmt` gives this formatting, even though it's ugly
        let config_dir =
            ConfigValue::from_env_or_builtin("FONTPM_CONFIG_DIR", || {
                dirs::preference_dir()
                    .expect("preference_dir required")
                    .join("fontpm")
            })
            .map(to_arc_path);
        let fontpm_config_file =
            ConfigValue::from_env_or_builtin("FONTPM_CONFIG_FILE", || {
                config_dir.resolved.join("fontpm.toml")
            })
            .map(to_arc_path);
        Self {
            config_dir,
            fontpm_config_file,
        }
    }
}

fn to_arc_path(path: PathBuf) -> Arc<Path> {
    Arc::from(path)
}

#[derive(Clone, Debug)]
pub struct ConfigValue<T> {
    pub source: ConfigValueSource,
    pub resolved: T,
}
impl<T> ConfigValue<T> {
    pub fn env(
        variable: &'static str,
        transform: impl FnOnce(OsString) -> T,
    ) -> Option<Self> {
        if let Some(value) = std::env::var_os(variable) {
            Some(Self {
                source: ConfigValueSource::Environment(variable),
                resolved: transform(value),
            })
        } else {
            None
        }
    }
    pub fn from_env(variable: &'static str) -> Option<Self>
    where
        T: From<OsString>,
    {
        Self::env(variable, T::from)
    }
    pub fn builtin(value: T) -> Self {
        Self {
            source: ConfigValueSource::Builtin,
            resolved: value,
        }
    }

    pub fn from_env_or_builtin(
        variable: &'static str,
        default: impl FnOnce() -> T,
    ) -> Self
    where
        T: From<OsString>,
    {
        Self::from_env(variable).unwrap_or_else(|| Self::builtin(default()))
    }

    pub fn map<O>(self, f: impl FnOnce(T) -> O) -> ConfigValue<O> {
        ConfigValue {
            resolved: f(self.resolved),
            source: self.source,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConfigValueSource {
    Builtin,
    Environment(&'static str),
    File(Arc<Path>),
}
impl fmt::Display for ConfigValueSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Builtin => {
                write!(f, "builtin default")
            }
            Self::File(file) => {
                write!(f, "file \"{}\"", file.display())
            }
            Self::Environment(variable) => {
                write!(f, "environment variable `{}`", variable)
            }
        }
    }
}
