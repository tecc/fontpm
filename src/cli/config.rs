//! Configuration management command.
//!
//! # `fontpm config where`
//! Prints the path to the configuration file.
//!
//! # `fontpm config print`
//! Prints the configuration that the program is using in a TOML format.

use crate::config::ConfigPaths;
use clap::{Args, Subcommand};
use console::style;
use std::io::Write;

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    Where,
}

pub fn run(args: ConfigArgs) {
    match args.command {
        ConfigCommand::Where => {
            let mut console = console::Term::buffered_stdout();
            let paths = ConfigPaths::load();
            let _ = writeln!(
                console,
                "Config directory: {} ({})",
                style(paths.config_dir.resolved.display()).bold(),
                paths.config_dir.source
            );
            let _ = writeln!(
                console,
                "fontpm.toml path: {} ({})",
                style(paths.fontpm_config_file.resolved.display()).bold(),
                paths.config_dir.source
            );
            let _ = console.flush();
        }
    }
}
