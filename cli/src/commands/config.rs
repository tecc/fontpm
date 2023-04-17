use clap::{arg, ArgAction, ArgMatches, Command, Subcommand, FromArgMatches};
use fontpm_api::{info, ok, Error as FError};
use crate::commands::{CommandAndRunner, Error};
use crate::config::{EntireConfig, FpmConfig};
use crate::runner;

pub const NAME: &str = "config";

const CMD_PRINT: &str = "print";

#[derive(Subcommand)]
enum ConfigCommand {
    Path {
        #[arg(long = "raw", help = "Tells FontPM to print the path without formatting nor newlines.")]
        raw: bool
    },
    Print {
        #[arg(long = "raw", help = "Tells FontPM to print the configuration file as TOML.")]
        raw: bool
    }
}

runner! { master_args =>
    // TODO: Setting configuration values
    let cmd = ConfigCommand::from_arg_matches(master_args)?;
    match cmd {
        ConfigCommand::Path { raw } => {
            let config_file = EntireConfig::config_file();
            if raw {
                print!("{}", config_file.display());
            } else {
                ok!("The path to the configuration file is {}", config_file.display());
            }
            Ok(None)
        },
        ConfigCommand::Print { raw } => {
            if raw {
                let config = EntireConfig::load()?;
                let toml = toml::ser::to_string_pretty(&config)
                    .map_err(|v| Error::API(FError::Serialisation(v.to_string())))?;
                print!("{}", toml);
                // Ok(None)
            } else {
                let config = FpmConfig::load()?;

                macro_rules! config_write_stringify {
                    (option:$kind:tt$(:$kind_extra:tt)* $value:expr; default $dkind:tt$(:$dkind_extra:tt)* $default:expr;) => {
                        if let Some(value) = &$value {
                            config_write_stringify!($kind$(:$kind_extra)* value;)
                        } else {
                            let strinigifed = config_write_stringify!($dkind$(:$dkind_extra)* $default;);
                            format!("<not set> [default: {}]", strinigifed)
                        }
                    };
                    (option:$kind:tt$(:$kind_extra:tt)* $value:expr;) => {
                        if let Some(value) = &$value {
                            config_write_stringify!($kind$(:$kind_extra)* value;)
                        } else {
                            "<not set>".to_string()
                        }
                    };
                    (array:$kind:tt$(:$kind_extra:tt)* $value:expr;) => {{
                        let stringified: Vec<String> = $value.iter().map(|v| {
                            config_write_stringify!($kind$(:$kind_extra)* v;)
                        }).collect();
                        let stringified = stringified.join(", ");
                        format!("[{}] {}", $value.len(), stringified)
                    }};
                    (path $value:expr;) => {{
                        let value = &$value;
                        let path: &::std::path::Path = value.as_ref();
                        format!("{}", path.display())
                    }};
                    (string $value:expr;) => {
                        format!("\"{}\"", $value)
                    };
                }
                macro_rules! config_write {
                    ($id:literal => $kind:tt$(:$kind_extra:tt)* $value:expr; $($extra:tt)*) => {{
                        info!("{}: {}", $id, config_write_stringify!($kind$(:$kind_extra)* $value; $($extra)*));
                    }};
                    ($id:literal => $kind:tt$(:$kind_extra:tt)* $value:expr) => {
                        config_write!($id => $kind$(:$kind_extra)* $value;)
                    }
                }
                // might've overcomplicated this severely but i like it
                config_write!("fontpm.enabled_sources" => array:string config.enabled_sources);
                config_write!("fontpm.cache_dir" => option:path config.cache_dir; default path config.cache_dir(););
                config_write!("fontpm.font_install_dir" => option:path config.font_install_dir; default path config.font_install_dir(););
            }

            Ok(None)
        }
    }
}

pub fn command() -> CommandAndRunner {
    let command = Command::new(NAME)
        .about("Utilities for reading and updating the configuration.")
        .subcommand_required(true);
    let command = ConfigCommand::augment_subcommands(command);
    return CommandAndRunner {
        description: command,
        runner: Box::new(runner)
    }
}