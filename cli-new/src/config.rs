//! Configuration utilities.
//!
//! # Config directory (`config_dir`)
//!
//! Default location: `{dirs::preference_dir()}/fontpm`
//!
//! # Main config file (`config_file`)
//!
//! Default location: `{config_dir}/fontpm.toml`

use crate::cli::CliContext;
use anyhow::Context;
use reqwest::header::HeaderValue;
use serde::de::{DeserializeOwned, IntoDeserializer};
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Config {
    pub paths: ConfigPaths,
    pub fontpm_toml: FileTable,
    pub fontpm: FontpmConfig,
}
/// Helper macro to load configuration values.
macro_rules! config {
    () => {};
    (
        @ $x:ident
        $fn_name:ident ( $($fn_args:tt)* )
        $(, $($remaining:tt)* )?
    ) => {
        config!(# $x $fn_name $($fn_args)* );
        $(config!(@ $x $( $remaining )*);)?
    };
    (
        # $x:ident
        env
        $variable:literal
        $(, then: $and_then_fn:expr)?
    ) => {
        let $x = $x.or_else(|| {
            let env = ConfigValue::from_env_as($variable);
            $(
            let env = env.inner_map($and_then_fn);
            )?
            env
        });
    };
    (
        # $x:ident
        toml
        $file_table:expr,
        $key:literal
        $(, then: $and_then_fn:expr )?
    ) => {
        let $x = $x.or_else(|| {
            let file_table = &($file_table);
            let value = file_table.get_key($key);
            $(
                let value = {
                    let f = $and_then_fn;
                    let wrapped = |a| f(a).map_err(anyhow::Error::from);
                    value.inner_map(|result| result.and_then(wrapped))
                };
            )?
            value
        });
    };
    (
        # $x:ident
        builtin
        $value:expr
    ) => {
        let $x = $x.unwrap_or_else(|| ConfigValue::builtin($value));
    };
    (
        # $x:ident
        map
        $map_fn:expr
    ) => {
        let $x = $x.map($map_fn);
    };
    (
        # $x:ident
        inner_map
        $($map_fn:expr),*
    ) => {
        $( let $x = $x.inner_map($map_fn); )*
    };
    (
        @ $x:ident
    ) => {};
    (
        # $($remaining:tt)*
    ) => {
        compile_error!(concat!("Unexpected tokens: `", $(stringify!($remaining))*, "`"))
    };
    (
        $($token:tt)*
    ) => {
        {
            use $crate::config::config;
            let value = ConfigValue::builtin(None);
            config!(@ value $($token)* );
            value
        }
    };
}
pub(crate) use config;

const DEFAULT_CONFIGURATION: &'static str = include_str!("../fontpm.toml");
const DEFAULT_HTTP_USER_AGENT: HeaderValue =
    HeaderValue::from_static(concat!("fontpm/", env!("CARGO_PKG_VERSION")));

impl Config {
    pub fn load(cli: &CliContext) -> anyhow::Result<Self> {
        let paths = ConfigPaths::load();

        let fontpm_toml = if paths.fontpm_config_file.resolved.exists() {
            FileTable::load_file(cli, &paths.fontpm_config_file.resolved)?
        } else {
            if !cli.modify_files {
                let _ = writeln!(
                    cli.warn(),
                    "The configuration file does not exist, but none will be created"
                );
            } else {
                let _ = writeln!(
                    cli.warn(),
                    "The configuration file does not exist, so a default will be written to it"
                );
                if let Err(e) = crate::util::create_parent_all(
                    &paths.fontpm_config_file.resolved,
                ) {
                    let _ = writeln!(
                        cli.warn(),
                        "Could not create parent for default configuration: {}",
                        e
                    );
                } else if let Err(e) = std::fs::write(
                    &paths.fontpm_config_file.resolved,
                    DEFAULT_CONFIGURATION,
                ) {
                    let _ = writeln!(
                        cli.warn(),
                        "Could not write default configuration: {}",
                        e
                    );
                }
            }

            FileTable::new(paths.fontpm_config_file.resolved.clone())
        };

        let fontpm = FontpmConfig::load(cli, &fontpm_toml)?;

        Ok(Self {
            paths,
            fontpm_toml,
            fontpm,
        })
    }
}

pub struct FontpmConfig {
    /// Path to the file store.
    ///
    /// Sources:
    /// 1. Environment variable: `FONTPM_STORE_DIR`
    /// 2. `fontpm.toml` key: `fontpm.store_dir`
    /// 3. Default: `{dirs::cache_dir()}/fontpm/store`
    pub store_dir: ConfigValue<Arc<Path>>,
    /// Path to a directory that FontPM should use for storing downloads
    /// temporarily, before moving them to the object store.
    ///
    /// This is located in the same directory as the store in case the store
    /// directory and temporary directory are located on different filesystems.
    ///
    /// Sources:
    /// 1. Environment variable: `FONTPM_DOWNLOAD_TMP_DIR`
    /// 2. `fontpm.toml` key: `fontpm.download_tmp_dir`
    /// 3. Default: `{store_dir}/cache`
    pub store_tmp_dir: ConfigValue<Arc<Path>>,
    /// Enabled sources (see [`crate::source`]).
    ///
    /// Sources:
    /// 1. Environment variable: `FONTPM_ENABLED_SOURCES` as a comma-separated list
    /// 2. `fontpm.toml` key: `fontpm.enabled_sources`
    /// 3. Default: Google Fonts, if available.
    pub enabled_sources: ConfigValue<Arc<[crate::source::SourceId]>>,
    pub http: FontpmHttpConfig,
}
pub struct FontpmHttpConfig {
    /// User agent to use in HTTP requests.
    ///
    /// Sources:
    /// 1. Environment variable: `FONTPM_HTTP_USER_AGENT`
    /// 2. `fontpm.toml` key: `fontpm.user_agent`
    /// 3. Default: `fontpm/{CARGO_PKG_VERSION}`.
    pub user_agent: ConfigValue<HeaderValue>,
}
impl FontpmConfig {
    pub fn load(
        ctx: &CliContext,
        fontpm_toml: &FileTable,
    ) -> anyhow::Result<Self> {
        let store_dir = config!(
            env("FONTPM_STORE_DIR", then: Ok),
            toml(fontpm_toml, "fontpm.store_dir"),
            builtin(Ok(dirs::cache_dir()
                .expect("cache_dir must exist")
                .join("fontpm/store"))),
        )
        .fail(ctx, "store directory")?
        .map(util::to_arc_path);

        let download_tmp_dir = config!(
            env("FONTPM_DOWNLOAD_TMP_DIR", then: Ok),
            toml(fontpm_toml, "fontpm.download_tmp_dir"),
            builtin(Ok(store_dir.resolved.join("tmp")))
        )
        .fail(ctx, "temporary directory for downloads")?
        .map(util::to_arc_path);

        let enabled_sources = config!(
            env("FONTPM_ENABLED_SOURCES", then: util::comma_separated_list_fromstr),
            toml(fontpm_toml, "fontpm.enabled_sources"),
            builtin(Ok(Arc::from(crate::source::DEFAULT_ENABLED_SOURCES)))
        )
        .fail(ctx, "enabled sources")?;

        let user_agent = config!(
            env("FONTPM_HTTP_USER_AGENT", then: util::to_header_value),
            inner_map(anyhow::Result::from),
            toml(fontpm_toml, "fontpm.http.user_agent", then: |x: String| HeaderValue::try_from(x)),
            builtin(Ok(DEFAULT_HTTP_USER_AGENT))
        )
        .fail(ctx, "HTTP user agent")?;

        Ok(Self {
            store_dir,
            store_tmp_dir: download_tmp_dir,
            enabled_sources,
            http: FontpmHttpConfig { user_agent },
        })
    }
}

/// Utilities for loading configuration values.
pub mod util {
    use super::*;

    pub fn comma_separated_list_fromstr<T>(
        input: OsString,
    ) -> anyhow::Result<Arc<[T]>>
    where
        T: std::str::FromStr,
        anyhow::Error: From<T::Err>,
    {
        comma_separated_list(input, |a| T::from_str(a))
    }
    pub fn comma_separated_list<T, E>(
        input: OsString,
        mut f: impl FnMut(&str) -> Result<T, E>,
    ) -> anyhow::Result<Arc<[T]>>
    where
        anyhow::Error: From<E>,
    {
        let mut vec = vec![];
        let input = input.into_string().map_err(|e| {
            anyhow::anyhow!(
                "input string could not be converted to a Rust string"
            )
        })?;
        for entry in input.split(',') {
            vec.push(f(entry)?);
        }
        Ok(vec.into())
    }
    pub fn to_header_value(input: OsString) -> anyhow::Result<HeaderValue> {
        HeaderValue::from_bytes(input.as_encoded_bytes())
            .context("converting OsString to Hea")
    }
    pub fn os_to_url(input: OsString) -> anyhow::Result<reqwest::Url> {
        let string = String::from_utf8(input.into_encoded_bytes())?;
        Ok(string.parse()?)
    }
    pub fn os_to_string(input: OsString) -> String {
        input.to_string_lossy().into_owned()
    }
    pub fn to_arc_str(value: String) -> Arc<str> {
        Arc::from(value)
    }
    pub fn to_arc_path(path: PathBuf) -> Arc<Path> {
        Arc::from(path)
    }
}

pub struct ConfigPaths {
    /// Path to the configuration directory.
    ///
    /// Sources:
    /// 1. Environment variable: `FONTPM_CONFIG_DIR`
    /// 2. Default: `{dirs::preference_dir()}/fontpm`
    ///
    /// Example: `/home/Alice/.config/fontpm`
    pub config_dir: ConfigValue<Arc<Path>>,
    /// Path to the main configuration file, `fontpm.toml`.
    ///
    /// Sources:
    /// 1. Environment variable: `FONTPM_CONFIG_FILE`
    /// 2. Default: `{config_dir}/fontpm.toml`.
    ///
    /// Example: `/home/Alice/.config/fontpm/fontpm.toml`.
    pub fontpm_config_file: ConfigValue<Arc<Path>>,
}

impl ConfigPaths {
    pub fn load() -> Self {
        let config_dir = config!(
            env("FONTPM_CONFIG_DIR"),
            builtin(
                dirs::preference_dir()
                    .expect("preference_dir required")
                    .join("fontpm")
            ),
            map(util::to_arc_path)
        );
        let fontpm_config_file = config!(
            env("FONTPM_CONFIG_FILE"),
            builtin(config_dir.resolved.join("fontpm.toml")),
            map(util::to_arc_path)
        );
        Self {
            config_dir,
            fontpm_config_file,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConfigValue<T> {
    pub source: ConfigValueSource,
    pub resolved: T,
}

impl<T> ConfigValue<T> {
    pub fn from_env(variable: &'static str) -> ConfigValue<Option<OsString>> {
        ConfigValue {
            source: ConfigValueSource::Environment(variable),
            resolved: std::env::var_os(variable),
        }
    }
    pub fn from_env_as(variable: &'static str) -> ConfigValue<Option<T>>
    where
        T: From<OsString>,
    {
        Self::from_env(variable).inner_map(T::from)
    }
    pub fn builtin(value: T) -> Self {
        Self {
            source: ConfigValueSource::Builtin,
            resolved: value,
        }
    }

    pub fn map<O>(self, f: impl FnOnce(T) -> O) -> ConfigValue<O> {
        ConfigValue {
            resolved: f(self.resolved),
            source: self.source,
        }
    }
}

impl<T, E> ConfigValue<Result<T, E>> {
    pub fn inner_map<Q>(
        self,
        transform: impl FnOnce(T) -> Q,
    ) -> ConfigValue<Result<Q, E>> {
        ConfigValue {
            source: self.source,
            resolved: self.resolved.map(transform),
        }
    }
    pub fn inner_map_err<Q>(
        self,
        transform: impl FnOnce(E) -> Q,
    ) -> ConfigValue<Result<T, Q>> {
        ConfigValue {
            source: self.source,
            resolved: self.resolved.map_err(transform),
        }
    }
    pub fn transpose(self) -> Result<ConfigValue<T>, E> {
        match self.resolved {
            Ok(resolved) => Ok(ConfigValue {
                source: self.source,
                resolved,
            }),
            Err(e) => Err(e),
        }
    }

    pub fn fail(self, cli: &CliContext, key: &str) -> Result<ConfigValue<T>, E>
    where
        E: std::fmt::Debug,
    {
        match self.resolved {
            Ok(x) => Ok(ConfigValue {
                source: self.source,
                resolved: x,
            }),
            Err(e) => {
                let _ = writeln!(
                    cli.error(),
                    "Could not load {} from {}: {:?}",
                    key,
                    self.source,
                    e
                );
                Err(e)
            }
        }
    }
}
impl<T> ConfigValue<Option<T>> {
    pub fn inner_map<Q>(
        self,
        transform: impl FnOnce(T) -> Q,
    ) -> ConfigValue<Option<Q>> {
        ConfigValue {
            source: self.source,
            resolved: self.resolved.map(transform),
        }
    }
    pub fn or_else(self, f: impl FnOnce() -> ConfigValue<Option<T>>) -> Self {
        if self.resolved.is_some() {
            self
        } else {
            f()
        }
    }
    pub fn ok_or_else<E>(
        self,
        error: impl FnOnce() -> E,
    ) -> ConfigValue<Result<T, E>> {
        match self.resolved {
            Some(inner) => ConfigValue {
                source: self.source,
                resolved: Ok(inner),
            },
            None => ConfigValue {
                source: self.source,
                resolved: Err(error()),
            },
        }
    }
    pub fn unwrap_or_else(
        self,
        default: impl FnOnce() -> ConfigValue<T>,
    ) -> ConfigValue<T> {
        if let Some(resolved) = self.resolved {
            ConfigValue {
                source: self.source,
                resolved,
            }
        } else {
            default()
        }
    }
}

pub struct FileTable {
    pub document: toml_edit::DocumentMut,
    pub path: Arc<Path>,
}
impl FileTable {
    pub fn new(path: Arc<Path>) -> Self {
        Self {
            document: toml_edit::DocumentMut::new(),
            path,
        }
    }
    pub fn load_file(
        cli: &CliContext,
        path: &Arc<Path>,
    ) -> anyhow::Result<Self> {
        let config_contents = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(e) => {
                let _ = writeln!(
                    cli,
                    "Could not read configuration file {}: {}",
                    path.display(),
                    e
                );
                return Err(e.into());
            }
        };
        Self::load_str(cli, path, &config_contents)
    }
    pub fn load_str(
        cli: &CliContext,
        path: &Arc<Path>,
        content: &str,
    ) -> anyhow::Result<Self> {
        let document = match content.parse::<toml_edit::DocumentMut>() {
            Ok(doc) => doc,
            Err(e) => {
                let _ = writeln!(
                    cli,
                    "Could not parse file {} as TOML: {}",
                    path.display(),
                    e
                );
                return Err(e.into());
            }
        };
        Ok(Self {
            document,
            path: path.clone(),
        })
    }

    pub fn get_key<T>(
        &self,
        config_key: &'static str,
    ) -> ConfigValue<Option<anyhow::Result<T>>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        const DELIMITER: char = '.';

        let source = ConfigValueSource::TomlFile {
            path: self.path.clone(),
            key: config_key,
        };
        let mut current_table: &dyn toml_edit::TableLike =
            self.document.as_table();
        let key = if let Some((table_prefix, final_key)) =
            config_key.rsplit_once(DELIMITER)
        {
            for (idx, part) in table_prefix.split(DELIMITER).enumerate() {
                let Some(table) = current_table.get(part) else {
                    return ConfigValue {
                        source,
                        resolved: None,
                    };
                };
                let Some(table) = table.as_table_like() else {
                    return ConfigValue {
                        source,
                        resolved: Some(Err(FileTableError::ExpectedTable {
                            key: config_key,
                            table: idx,
                        }
                        .into())),
                    };
                };
                current_table = table;
            }
            final_key
        } else {
            config_key
        };

        let Some(value) = current_table.get(key) else {
            return ConfigValue {
                source,
                resolved: None,
            };
        };

        ConfigValue {
            source,
            resolved: match value.clone().into_value() {
                Ok(value) => Some(
                    T::deserialize(value.into_deserializer())
                        .context("failed to deserialize"),
                ),
                Err(_) => None,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileTableError {
    #[error("expected {key}'s {table}th key to be a table")]
    ExpectedTable { key: &'static str, table: usize },
    #[error("could not deserialize: {0}")]
    Deserialize(toml_edit::de::Error),
}

#[derive(Clone, Debug)]
pub enum ConfigValueSource {
    Builtin,
    Environment(&'static str),
    TomlFile { path: Arc<Path>, key: &'static str },
}
impl fmt::Display for ConfigValueSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Builtin => {
                write!(f, "builtin default")
            }
            Self::TomlFile { path, key } => {
                write!(f, "key \"{}\" in file \"{}\"", key, path.display())
            }
            Self::Environment(variable) => {
                write!(f, "environment variable `{}`", variable)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensure that the default configuration is a valid configuration.
    #[test]
    fn default_config_loads() {
        let context = CliContext::mock();
        let table = FileTable::load_str(
            &context,
            &PathBuf::new().into(),
            DEFAULT_CONFIGURATION,
        )
        .expect("Default configuration cannot be loaded as FileTable");
        FontpmConfig::load(&context, &table)
            .expect("FontpmConfig could not be loaded");
    }
}
