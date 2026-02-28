use crate::cli::{tri, tri_f, CliContext, GlobalOptions};
use crate::config::Config;
use crate::source::{SourceContext, SourceId, SourceSubcommand, Sources};
use clap::builder::PossibleValue;
use clap::{
    ArgMatches, Args, Command, Error, FromArgMatches, Subcommand, ValueEnum,
};
use std::ffi::OsString;
use std::process::ExitCode;
use std::sync::{Arc, LazyLock, OnceLock};

/// Source-specific commands
#[derive(Debug, Args)]
pub struct SourceArgs {
    #[command(flatten)]
    global: GlobalOptions,
    #[command(flatten)]
    opts: Opts,
}
#[derive(Debug, Args)]
struct Opts {
    /// List all registered sources.
    #[arg(short, long, exclusive = true)]
    list: Option<ListFilter>,
    /// The ID of the source to act on.
    #[arg(required_unless_present("list"))]
    source: Option<SourceId>,
    #[command(subcommand)]
    args: CliSubcommand,
}

#[derive(Debug)]
struct CliSubcommand(ArgMatches);

// Cheat to allow reuse of config
static CLI_CONTEXT: LazyLock<Arc<CliContext>> =
    LazyLock::new(|| Arc::new(CliContext::mock_stdout()));
static CONFIG: LazyLock<Arc<Config>> =
    LazyLock::new(|| Arc::new(Config::load(&CLI_CONTEXT).unwrap()));
static SOURCES: LazyLock<Sources> =
    LazyLock::new(|| Sources::create_enabled(&CLI_CONTEXT, &CONFIG).unwrap());

impl FromArgMatches for CliSubcommand {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        Ok(Self(matches.clone()))
    }
    fn update_from_arg_matches(
        &mut self,
        matches: &ArgMatches,
    ) -> Result<(), Error> {
        self.0 = matches.clone();
        Ok(())
    }
}

impl Subcommand for CliSubcommand {
    fn augment_subcommands(cmd: Command) -> Command {
        cmd.defer(|command| {
            command.subcommands(SOURCES.iter().map(|source| {
                let command = Command::new(source.id().canonical_name());
                source.augment(command)
            }))
        })
    }

    fn augment_subcommands_for_update(cmd: Command) -> Command {
        cmd.defer(|mut command| {
            for source in SOURCES.iter() {
                command = source.augment(command);
            }
            command
        })
    }

    fn has_subcommand(name: &str) -> bool {
        SOURCES.is_enabled(name)
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ListFilter {
    Enabled,
    Available,
}

pub fn run(args: SourceArgs) -> ExitCode {
    let cli = CliContext::new(&args.global);
    let config = tri_f!(::load_config, &cli);
    match args.opts {
        Opts {
            list: Some(filter), ..
        } => {
            todo!()
        }
        Opts {
            source: Some(source_id),
            mut args,
            ..
        } => {
            let Some(source) =
                Sources::create_specific(&cli, &config, &source_id)
            else {
                let _ = writeln!(
                    cli.error(),
                    "Source {} does not exist",
                    source_id
                );
                return ExitCode::FAILURE;
            };
            let mut source = tri!(
                source,
                cli.error(),
                "Could not create source {}",
                source_id
            );
            source.execute(cli, config, &mut args.0)
        }
        Opts { .. } => {
            unreachable!()
        }
    }
}
