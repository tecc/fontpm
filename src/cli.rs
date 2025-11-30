mod config;
mod install;
mod list;
mod refresh;
mod source;

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
    /// Avoid modifying files if at all possible.
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
    Source(source::SourceArgs),
}

pub fn run() -> ExitCode {
    let args = CliArgs::parse();

    match args.command {
        CliCommand::Config(args) => config::run(args),
        CliCommand::Install(args) => install::run(args),
        CliCommand::Refresh(args) => refresh::run(args),
        CliCommand::Source(args) => source::run(args),
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
            20,
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

/// Unwraps `value` from `Ok(value)`, or prints an error message and returns an
/// [`ExitCode`].
macro_rules! tri {
    ($expr:expr, $cli_out:expr, $fmt:literal $(, $fmt_args:expr)*) => {
        {
            match $expr {
                Ok(value) => value,
                Err(e) => {
                    let _ = writeln!(
                        $cli_out,
                        "{}: {}",
                        format_args!($fmt $(, $fmt_args)*),
                        e
                    );
                    return ExitCode::FAILURE;
                }
            }
        }
    };
}
/// Shorthand for common [`tri`] calls.
macro_rules! tri_f {
    (::load_config, $cli:expr) => {{
        let cli: &$crate::cli::CliContext = $cli;
        $crate::cli::tri!(
            $crate::config::Config::load(cli),
            cli.error(),
            "Could not load configuration"
        )
    }};
    (::create_sources, $cli:expr, $config:expr) => {{
        let cli: &$crate::cli::CliContext = $cli;
        let config: &$crate::config::Config = $config;
        $crate::cli::tri!(
            $crate::source::Sources::create_enabled(cli, config),
            cli.error(),
            "Could not load sources"
        )
    }};
    (::source_ctx, $cli:expr, $config:expr) => {{
        let cli: &Arc<$crate::cli::CliContext> = $cli;
        let config: &$crate::config::Config = $config;
        $crate::cli::tri!(
            $crate::source::SourceContext::new(cli.clone(), config)
                .map(Arc::new),
            cli.error(),
            "Could not create source context"
        )
    }};
    (::tokio, $cli:expr, $config:expr) => {{
        let cli: &$crate::cli::CliContext = $cli;
        let config: &$crate::config::Config = $config;
        $crate::cli::tri!(
            $crate::util::create_runtime(config),
            cli.error(),
            "Could not create Tokio runtime"
        )
    }};
}

pub(crate) use {tri, tri_f};
