mod config;
mod install;
mod list;
mod refresh;

use clap::{Args, Parser, Subcommand};
use console::{style, StyledObject};
use indicatif::{MultiProgress, ProgressDrawTarget};
use std::process::ExitCode;
use std::{fmt, io};

/// FontPM, the font package manager.
///
/// FontPM is a tool to manage fonts. Lorem ipsum dolores sit amet, put
/// something else here later.
///
/// # Installation
/// FontPM offers system-wide installation and user-local installation.
///
/// TODO: Project-local font management
#[derive(Debug, Parser)]
#[clap(name = "fontpm")]
#[command(max_term_width = 80)]
pub struct CliArgs {
    #[command(flatten)]
    pub global: GlobalOptions,
    /// The command to run.
    #[command(subcommand)]
    pub command: CliCommand,
}
/// Globally-available options.
///
/// To access these options in commands, add this struct as a field.
#[derive(Debug, Args)]
pub struct GlobalOptions {
    /// Simulate the changes without writing anything to disk.
    #[arg(long = "dry-run", global = true)]
    pub dry_run: bool,
    /// Increase the verbosity of the output.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}
#[derive(Debug, Subcommand)]
pub enum CliCommand {
    Config(config::ConfigArgs),
    Install(install::InstallArgs),
    Refresh(refresh::RefreshArgs),
}

pub fn run() -> ExitCode {
    let args = CliArgs::parse();

    match args.command {
        CliCommand::Config(args) => config::run(args),
        CliCommand::Install(args) => install::run(args),
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
    mpb: Option<MultiProgress>,
}
impl CliContext {
    pub fn mock() -> Self {
        Self {
            term: console::Term::stdout(),
            modify_files: false,
            verbose: false,
            mpb: None,
        }
    }
    pub fn new(options: &GlobalOptions) -> Self {
        Self {
            term: console::Term::stderr(),
            modify_files: !options.dry_run,
            verbose: options.verbose,
            mpb: None,
        }
    }

    pub fn multiprogress(&mut self) -> MultiProgress {
        let mpb = MultiProgress::with_draw_target(ProgressDrawTarget::term(
            self.term.clone(),
            5,
        ));
        self.mpb = Some(mpb.clone());
        mpb
    }

    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        if let Some(mpb) = &self.mpb {
            let s = format!("{}", args);
            mpb.println(&s)
        } else {
            io::Write::write_fmt(&mut &self.term, args)
        }
    }

    pub fn debug(&'_ self) -> PrefixedOutput<'_> {
        PrefixedOutput {
            prefix: style("debug:").white().dim(),
            ctx: &self,
            noop: !self.verbose,
        }
    }

    pub fn ok(&'_ self) -> PrefixedOutput<'_> {
        PrefixedOutput {
            prefix: style("ok!").bold().green(),
            ctx: &self,
            noop: false,
        }
    }

    pub fn warn(&'_ self) -> PrefixedOutput<'_> {
        PrefixedOutput {
            prefix: style("warn:").bold().yellow(),
            ctx: &self,
            noop: false,
        }
    }
    pub fn warn_v(&'_ self) -> PrefixedOutput<'_> {
        PrefixedOutput {
            prefix: style("warn[verbose]:").bold().yellow(),
            ctx: &self,
            noop: !self.verbose,
        }
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
        if self.noop {
            return Ok(());
        }
        write!(self.ctx, "{} {}", self.prefix, args)
    }
    fn with_noop(mut self, value: bool) -> Self {
        self.noop = value;
        self
    }
}
