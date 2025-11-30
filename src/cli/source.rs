use crate::cli::{tri, tri_f, CliContext, GlobalOptions};
use crate::config::Config;
use crate::source::{SourceId, SourceSubcommand, Sources};
use clap::builder::PossibleValue;
use clap::{Args, Subcommand, ValueEnum};
use std::ffi::OsString;
use std::process::ExitCode;

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
    args: Option<SubcommandArgs>,
}
// It looks like this because otherwise the help message doesn't look correct
#[derive(Debug, Subcommand)]
enum SubcommandArgs {
    #[command(external_subcommand)]
    External(Vec<OsString>),
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
            args,
            ..
        } => {
            let args = args.unwrap_or(SubcommandArgs::External(vec![]));
            let SubcommandArgs::External(args) = args;

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

            source.execute_command(SourceSubcommand { cli, config, args })
        }
        Opts { .. } => {
            unreachable!()
        }
    }
}
