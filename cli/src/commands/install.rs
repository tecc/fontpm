use std::task::Context;
use clap::{arg, ArgMatches, Command};
use fontpm_api::error;
use crate::commands::{self, CommandAndRunner, Error};

pub const NAME: &str = "install";

fn runner(args: &ArgMatches) -> commands::Result {
    Err(Error::TODO(None))
}

pub fn command() -> CommandAndRunner {
    return CommandAndRunner {
        description: Command::new(NAME)
            .about("Install a font.")
            .args(vec![
                arg!(--source <source> "Selects the source to install the font from.")
                    .long_help("Selects the source to install the font from. If the font is not available from the source, the command fails.")
            ])
        ,
        runner
    };
}