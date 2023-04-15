use clap::{arg, ArgAction, ArgMatches, Command, Subcommand, FromArgMatches};
use fontpm_api::{info, Error as FError};
use crate::commands::{CommandAndRunner, Error};
use crate::runner;

pub const NAME: &str = "config";

const CMD_PRINT: &str = "print";

#[derive(Subcommand)]
enum ConfigCommand {
    Print {
        raw: bool
    }
}

runner! { master_args =>
    // TODO: Setting configuration values
    let cmd = ConfigCommand::from_arg_matches(master_args)?;
    match cmd {
        ConfigCommand::Print { raw } => {
            if raw {
                let config = crate::config::EntireConfig::load()?;
                let toml = toml::ser::to_string_pretty(&config)
                    .map_err(|v| Error::API(FError::Serialisation(v.to_string())))?;
                print!("{}", toml);
                return Ok(None)
            } else {
                let config = crate::config::FpmConfig::load()?;

                macro_rules! config_write_stringify {
                    (array $value:expr) => {
                        format!("[{}] {}", $value.len(), $value.join(", "))
                    };
                    (path $value:expr) => {
                        format!("{}", $value.to_string_lossy())
                    };
                    (option:$kind:tt$(:$kind_extra:tt)* $value:expr) => {
                        if let Some(v) = $value {
                            config_write_stringify!($kind$(:$kind_extra)* v)
                        } else {
                            format!("<not set>")
                        }
                    };
                }
                macro_rules! config_write {
                    ($id:literal => $kind:tt$(:$kind_extra:tt)* $value:expr) => {
                        info!("{}: {}", $id, config_write_stringify!($kind$(:$kind_extra)* $value))
                    };
                }

                // might've overcomplicated this severely but i like it
                config_write!("fontpm.enabled_sources" => array config.enabled_sources);
                config_write!("fontpm.cache_dir" => option:path config.cache_dir);
                config_write!("fontpm.font_install_dir" => option:path config.font_install_dir);

            }

            Ok(None)
        }
    }
}

pub fn command() -> CommandAndRunner {
    return CommandAndRunner {
        description: Command::new(NAME)
            .about("Utilities for reading and updating the configuration.")
            .subcommands(vec![
                Command::new(CMD_PRINT)
                    .about("Prints the in-memory configuration")
                    .args(vec![
                        arg!(--raw "Print the configuration as TOML")
                            .action(ArgAction::SetTrue)
                    ])
            ])
            .subcommand_required(true),
        runner: Box::new(runner)
    }
}