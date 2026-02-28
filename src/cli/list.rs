//! List all installed fonts.
//!
//! # Outdated font detection
//!
//! If a font was installed from a source that supports versioning, tell the
//! user that their installed font is potentially out-of-date.

use crate::cli::{tri, tri_f, CliContext, GlobalOptions};
use crate::platform::Platform;
use crate::util::font::FontReference;
use chrono::{DateTime, Utc};
use clap::Args;
use console::style;
use futures_util::StreamExt;
use std::process::ExitCode;
use std::{fmt, io};

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

pub fn run(args: ListArgs) -> ExitCode {
    let cli = CliContext::new(&args.global);
    let config = tri_f!(::load_config, &cli);
    let platform = tri!(
        Platform::load(&cli, &config),
        &cli,
        "could not load platform"
    );
    let runtime = tri_f!(::tokio, &cli, &config);
    // TODO: Version check
    // let sources = tri_f!(::create_sources, &cli, &config);

    writeln!(cli, "{:?}", args);

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

            let _ = writeln!(
                cli,
                "{}",
                style("== LOCALLY INSTALLED FONTS ==").bold()
            );
            for font in fonts {
                let _ = write_font(&cli, &font, None);
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
    new_version: Option<(&str, DateTime<Utc>)>,
) -> io::Result<()> {
    write!(
        cli,
        "{}: version {} ({})",
        style(&font.identifier.id).yellow(),
        font.version,
        font.timestamp.date_naive()
    )?;
    if let Some((version, timestamp)) = new_version {
        write!(
            cli,
            "{}",
            style(format_args!(
                "<-- new version available: {} ({})",
                style(version).bold(),
                timestamp.date_naive()
            ))
            .green()
        )?;
    }
    writeln!(cli)
}
