//! List all installed fonts.
//!
//! # Outdated font detection
//!
//! If a font was installed from a source that supports versioning, tell the
//! user that their installed font is potentially out-of-date.

use crate::cli::{tri, tri_f, CliContext, GlobalOptions};
use crate::platform::Platform;
use crate::util::font::{FontReference, FontSpec};
use clap::Args;
use console::{style, Style};
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use std::io;
use std::process::ExitCode;
use std::sync::Arc;

/// List fonts installed through FontPM.
#[derive(Debug, Args)]
pub struct ListArgs {
    #[command(flatten)]
    global: GlobalOptions,
    /// Show locally installed fonts.
    ///
    /// If no filter is explicitly specified, this is enabled.
    #[arg(short = 'l', long)]
    show_local: Option<Option<bool>>,
    /// Show globally installed fonts.
    #[arg(short = 'g', long)]
    show_global: Option<Option<bool>>,
}

#[derive(Default)]
struct ItemMetadata {
    new_version: Option<FontReference>,
}
pub fn run(args: ListArgs) -> ExitCode {
    let cli = Arc::new(CliContext::new(&args.global));
    let config = tri_f!(::load_config, &cli);
    let platform = tri!(
        Platform::load(&cli, &config),
        &cli,
        "could not load platform"
    );
    let runtime = tri_f!(::tokio, &cli, &config);
    let source_context = tri_f!(::source_ctx, &cli, &config);
    let sources = tri_f!(::create_sources, &cli, &config);

    let show_local = args
        .show_local
        .unwrap_or_else(|| Some(args.show_global.is_none()))
        .unwrap_or(true);

    runtime.block_on(async {
        let mut exit = ExitCode::SUCCESS;

        if is_true(args.show_global) {
            // TODO
            let _ = writeln!(
                cli.warn(),
                "Globally installed fonts are not yet implemented"
            );
        }
        'local: {
            if !show_local {
                break 'local;
            }
            let fonts = match platform.list_installed_fonts_local(&cli).await {
                Ok(x) => x,
                Err(e) => {
                    let _ = writeln!(
                        cli.warn(),
                        "Could not list locally installed fonts: {}",
                        e
                    );
                    exit = ExitCode::FAILURE;
                    break 'local;
                }
            };

            let mut fonts = fonts.into_iter().map(|reference| (reference, ItemMetadata::default()))
                .collect::<Vec<_>>();
            tokio::pin!(fonts);

            #[derive(Default)]
            struct GetMetadataInfo {
                failed_update_check: bool
            }
            let tasks = FuturesUnordered::new();
            for (font, metadata) in fonts.iter_mut() {
                tasks.push(async {
                    let mut info = GetMetadataInfo::default();
                    'resolve: {
                        if let Some(source) = sources.get(&font.identifier.source) {
                            let spec = &FontSpec {
                                source: Some(font.identifier.source.clone()),
                                id: font.identifier.id.clone(),
                            };
                            let _ = writeln!(cli.debug(), "checking for new version of {}", spec);
                            let resolved = match source.resolve_font(&source_context, spec).await {
                                Ok(x) => x,
                                Err(e) => {
                                    info.failed_update_check = true;
                                    let _ = writeln!(cli.warn_v(), "{} failed to resolve: {:?}", spec, e);
                                    break 'resolve
                                }
                            };

                            let current_max = font.timestamp;
                            for resolution in resolved {
                                if resolution.reference.timestamp > current_max {
                                    metadata.new_version = Some(resolution.reference)
                                }
                            }
                        }
                    }
                    info
                });
            }
            let info = tasks.fold(GetMetadataInfo::default(), async |a, b| {
                GetMetadataInfo {
                    failed_update_check: a.failed_update_check || b.failed_update_check,
                }
            })
                .await;

            let _ = writeln!(
                cli,
                "{}",
                style("== LOCALLY INSTALLED FONTS ==").bold()
            );

            let mut update_available = false;
            for (font, metadata) in fonts.iter() {
                update_available = update_available || metadata.new_version.is_some();
                let _ = write_font(&cli, font, metadata);
            }

            if info.failed_update_check {
                let _ = writeln!(cli.warn(), "Checking for updates failed for some fonts (run with verbose logging for more details)");
            }
            let _ = writeln!(cli);
            if update_available {
                // TODO: Add update command
                let _ = writeln!(cli.note(), "Some fonts have newer versions available.");
            }
        }

        exit
    })
}

fn is_true(flag: Option<Option<bool>>) -> bool {
    flag.unwrap_or(Some(false)).unwrap_or(true)
}

fn write_font(
    cli: &CliContext,
    font: &FontReference,
    metadata: &ItemMetadata,
) -> io::Result<()> {
    write!(
        cli,
        "{}: version {} ({})",
        style(&font.identifier.id).yellow(),
        font.version,
        font.timestamp.date_naive()
    )?;
    if let Some(reference) = &metadata.new_version {
        const STYLE: Style = Style::new().green();
        write!(
            cli,
            "{}{} {}",
            STYLE.apply_to(" <-- new version available: "),
            STYLE.bold().apply_to(&reference.version),
            STYLE.apply_to(format_args!(
                "(released {})",
                &reference.timestamp.date_naive()
            )),
        )?;
    }
    writeln!(cli)
}
