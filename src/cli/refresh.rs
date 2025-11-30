use crate::cli::{tri_f, CliContext, GlobalOptions};
use crate::source::Refreshed;
use clap::Args;
use std::process::ExitCode;
use std::sync::Arc;

/// Refresh the local index.
#[derive(Debug, Args)]
pub struct RefreshArgs {
    #[command(flatten)]
    pub global: GlobalOptions,
}

pub fn run(args: RefreshArgs) -> ExitCode {
    let mut cli = CliContext::new(&args.global);
    let config = tri_f!(::load_config, &cli);
    let mut sources = tri_f!(::create_sources, &cli, &config);

    let mpb = cli.multiprogress();
    let cli = Arc::new(cli);

    let source_ctx = tri_f!(::source_ctx, &cli, &config);
    let runtime = tri_f!(::tokio, &cli, &config);

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

        if let Err(e) = source_ctx.store.save(&cli).await {
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
