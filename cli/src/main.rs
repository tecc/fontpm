pub mod commands;
mod output_impl;
mod host_impl;
mod sources;
mod config;
mod build_config;

#[macro_use]
extern crate lazy_static;

use clap::{arg, ArgAction, Command};
use clap::parser::ValueSource;
use semver::Version;
use fontpm_api::{error, ok};
use crate::commands::all_commands;
use crate::output_impl::OutputLevel;

pub const VERSION_STR: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    pub static ref VERSION: semver::Version = semver::Version::parse(VERSION_STR).unwrap();
}

#[tokio::main]
async fn main() {
    let subcommands = all_commands();

    let args = Command::new("FontPM")
        .bin_name("fontpm")
        .after_help("FontPM is a utility for managing fonts.")
        .after_long_help("FontPM is a utility for managing fonts. It provides different sources to find fonts from.")
        .author("tecc")
        .version(VERSION_STR)
        .subcommands(subcommands.values().map(|a| a.description.clone()))
        .subcommand_required(true)
        .args(vec![
            arg!(-s --silent "Silent output - will only print errors.")
                .global(true)
                .action(ArgAction::Count),
            arg!(-v --verbose "Verbose output - will print all messages. May be repeated.")
                .global(true)
                .action(ArgAction::Count)
                .conflicts_with("silent")
                ,
        ])
        .get_matches();

    {
        let output_level = if args.value_source("verbose") == Some(ValueSource::CommandLine) {
            let verbose = args.get_count("verbose") as u8;
            match verbose {
                0 => OutputLevel::Normal,
                1 => OutputLevel::Verbose,
                _ => OutputLevel::VeryVerbose
            }
        } else if args.value_source("silent") == Some(ValueSource::CommandLine) {
            let silent = args.get_count("silent") as u8;
            match silent {
                0 => OutputLevel::Normal,
                1 => OutputLevel::Silent,
                _ => OutputLevel::VerySilent
            }
        } else { OutputLevel::Normal };
        output_impl::init(output_level);
    }

    let (subcommand_name, subcommand_matches) = match args.subcommand() {
        Some(v) => v,
        None => unreachable!()
    };

    if let Some(subcommand) = subcommands.get(subcommand_name) {
        let result = (subcommand.runner)(subcommand_matches);

        match result {
            Ok(v) => {
                if let Some(message) = v {
                    ok!("{}", message);
                }
            },
            Err(e) => {
                error!("{}", e);
            }
        }
    } else {
        error!("No such subcommand");
    }

}
