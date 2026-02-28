//! Configuration management command.
//!
//! # `fontpm config where`
//! Prints the path to the configuration file.
//!
//! # `fontpm config print`
//! Prints the configuration that the program is using in a TOML format.

use crate::cli::{tri, tri_f, CliContext, GlobalOptions};
use crate::config::ConfigPaths;
use clap::{Args, Subcommand};
use console::style;
use std::io::Write;
use std::process::ExitCode;

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    Where,
    DebugPrint,
}

pub fn run(args: ConfigArgs) -> ExitCode {
    let cli = CliContext::new(&args.global);
    match args.command {
        ConfigCommand::Where => {
            let paths = ConfigPaths::load();
            let _ = writeln!(
                cli,
                "Config directory: {} ({})",
                style(paths.config_dir.resolved.display()).bold(),
                paths.config_dir.source
            );
            let _ = writeln!(
                cli,
                "fontpm.toml path: {} ({})",
                style(paths.fontpm_config_file.resolved.display()).bold(),
                paths.config_dir.source
            );
            ExitCode::SUCCESS
        }
        ConfigCommand::DebugPrint => {
            let config = tri_f!(::load_config, &cli);
            let _ = writeln!(cli, "{:#?}", config);
            ExitCode::SUCCESS
        }
    }
}
