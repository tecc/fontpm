mod config;
mod list;
mod refresh;

use clap::{Parser, Subcommand};
use console::{style, StyledObject};
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
    Refresh(refresh::RefreshArgs),
}

pub fn run() {
    let args = CliArgs::parse();

    match args.command {
        CliCommand::Config(args) => config::run(args),
        CliCommand::Refresh(args) => refresh::run(args),
    }
}

pub struct CliContext {
    term: console::Term,
    /// Whether or not any files should actually be modified: created, deleted,
    /// edited, etc.
    ///
    /// # Behaviour with `modify_files == false`
    ///
    /// No operation that would modify a file is permitted. To what extent it
    /// can be done, simulate it.
    pub modify_files: bool,
    pub verbose: bool,
}
impl CliContext {
    pub fn mock() -> Self {
        Self {
            term: console::Term::stdout(),
            modify_files: false,
            verbose: false,
        }
    }

    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        io::Write::write_fmt(&mut &self.term, args)
    }
    pub fn warn(&'_ self) -> PrefixedOutput<'_> {
        PrefixedOutput {
            prefix: style("warn:").bold().yellow(),
            ctx: &self,
            noop: false,
        }
    }
    pub fn warn_v(&'_ self) -> PrefixedOutput<'_> {
        self.warn().with_noop(!self.verbose)
    }

    pub fn error(&'_ self) -> PrefixedOutput<'_> {
        PrefixedOutput {
            prefix: style("error:").bold().red(),
            ctx: &self,
            noop: false,
        }
    }
}

pub struct PrefixedOutput<'a> {
    prefix: StyledObject<&'a str>,
    ctx: &'a CliContext,
    noop: bool,
}
impl<'a> PrefixedOutput<'a> {
    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        write!(self.ctx, "{} {}", self.prefix, args)?;
        self.ctx.write_fmt(args)
    }
    fn with_noop(mut self, value: bool) -> Self {
        self.noop = value;
        self
    }
}
