use crate::cli::{CliContext, GlobalOptions};
use crate::config::Config;
use crate::source::{Refreshed, SourceContext, Sources};
use crate::util::create_runtime;
use clap::Args;
use console::Term;
use indicatif::{MultiProgress, ProgressDrawTarget, ProgressStyle};
use std::process::ExitCode;
use std::sync::Arc;
use tokio::task::{JoinSet, LocalSet};

/// Refresh the local index.
#[derive(Debug, Args)]
pub struct RefreshArgs {
    #[command(flatten)]
    pub global: GlobalOptions,
}

pub fn run(args: RefreshArgs) -> ExitCode {
    let mut cli_context = CliContext::new(&args.global);
    let config = match Config::load(&cli_context) {
        Ok(x) => x,
        Err(e) => {
            let _ = writeln!(
                cli_context.error(),
                "Could not load configuration: {}",
                e
            );
            return ExitCode::FAILURE;
        }
    };

    let mut sources = match Sources::create_enabled(&cli_context, &config) {
        Ok(x) => x,
        Err(e) => {
            let _ = writeln!(
                cli_context.error(),
                "Could not create sources: {}",
                e
            );
            return ExitCode::FAILURE;
        }
    };
    let mpb = cli_context.multiprogress();
    let cli_context = Arc::new(cli_context);

    let source_ctx = match SourceContext::new(cli_context.clone(), &config) {
        Ok(x) => x,
        Err(e) => {
            let _ = writeln!(
                cli_context.error(),
                "Could not create source context: {}",
                e
            );
            return ExitCode::FAILURE;
        }
    };
    let source_ctx = Arc::new(source_ctx);
    let runtime = create_runtime(&config).unwrap();

    runtime.block_on(async move {
        let iter = sources.iter_mut().map(|source| async {
            let pb = source.create_progress_bar(&mpb);
            let result = source.refresh_index(&source_ctx, &pb).await;
            match &result {
                Err(e) => {
                    let _ = writeln!(
                        source_ctx.cli.error(),
                        "An error occurred whilst refreshing source '{}': {:?}",
                        source.id(),
                        e
                    );
                }
                _ => {}
            }
            result
        });

        let mut already_up_to_date = 0;
        let mut fresh = 0;
        let mut errored = 0;

        for result in futures::future::join_all(iter).await {
            match result {
                Ok(Refreshed::AlreadyUpToDate) => already_up_to_date += 1,
                Ok(Refreshed::Fresh) => fresh += 1,
                Err(_) => errored += 1,
            }
        }

        let mut exit_code = ExitCode::SUCCESS;

        if let Err(e) = source_ctx.store.save(&cli_context).await {
            let _ = writeln!(
                source_ctx.cli.error(),
                "Failed to save object store: {}",
                e,
            );
            exit_code = ExitCode::FAILURE;
        }

        let _ = mpb.clear();
        if errored > 0 {
            let _ = writeln!(
                source_ctx.cli.warn(),
                "{} source(s) failed to refresh (see above for details)",
                errored
            );
            exit_code = ExitCode::FAILURE;
        }
        let _ = writeln!(
            source_ctx.cli.ok(),
            "{} refreshed, {} already up-to-date",
            fresh,
            already_up_to_date
        );

        exit_code
    })
}
