mod config;
mod list;

use clap::{Parser, Subcommand};

/// FontPM, the font package manager.
///
/// FontPM is a tool to manage fonts. Lorem ipsum dolores sit amet, put
/// something else here later.
#[derive(Parser)]
#[clap(name = "fontpm")]
pub struct CliArgs {
    /// The command to run.
    #[command(subcommand)]
    pub command: CliCommand,
}
#[derive(Subcommand)]
pub enum CliCommand {}

pub fn run() {
    let args = CliArgs::parse();

    match args.command {}
}
