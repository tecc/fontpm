mod config;
mod list;

use clap::{Parser, Subcommand};
use console::style;
use std::{fmt, io};

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
pub enum CliCommand {
    Config(config::ConfigArgs),
}

pub fn run() {
    let args = CliArgs::parse();

    match args.command {
        CliCommand::Config(config) => config::run(config),
    }
}

pub struct CliContext {
    term: console::Term,
    pub modify_files: bool,
}
impl CliContext {
    pub fn mock() -> Self {
        Self {
            term: console::Term::stdout(),
            modify_files: false,
        }
    }

    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        io::Write::write_fmt(&mut &self.term, args)
    }
    pub fn warn(&'_ self) -> WarnOutput<'_> {
        WarnOutput(self)
    }
}

pub struct WarnOutput<'a>(&'a CliContext);
impl<'a> WarnOutput<'a> {
    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        write!(self.0, "{} ", style("warn:").bold().yellow(),)?;
        self.0.write_fmt(args)
    }
}
