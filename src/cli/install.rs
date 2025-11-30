use crate::cli::{tri, tri_f, CliContext, GlobalOptions};
use crate::util::font::{Download, FontSpec};
use anyhow::Context;
use clap::{Args, ValueEnum};
use console::style;
use futures_util::future::try_join_all;
use futures_util::FutureExt;
use futures_util::{StreamExt, TryFutureExt};
use indicatif::ProgressBar;
use std::process::ExitCode;
use std::sync::Arc;

/// Install a font globally (available for all users) or locally (only for
/// the current user).
#[derive(Debug, Args)]
pub struct InstallArgs {
    #[command(flatten)]
    pub global: GlobalOptions,
    #[command(flatten)]
    pub scope: ScopeArgs,
    /// Suppresses interactive prompts.
    ///
    /// If an undesirable situation that FontPM cannot resolve by itself occurs,
    /// FontPM will abort.
    #[arg(long)]
    pub noninteractive: bool,
    /// The fonts to install.
    ///
    /// These are specified as "font specs". Each font spec will result in only
    /// *one* font being installed.
    /// These specs are first resolved (finding every font that matches a spec),
    /// and then disambiguated (selecting one resolution for each spec).
    ///
    /// Font spec: `[<source>:]<font>`, where `<source>` refers to the source to
    /// get the font from, `<font>` is the ID of the font to install.
    #[arg(value_parser = str::parse::<FontSpec>, required = true)]
    pub fonts: Vec<FontSpec>,
}

#[derive(Copy, Clone, Debug, Args)]
#[group(multiple = false)]
pub struct ScopeArgs {
    /// Set the scope to install the fonts in.
    ///
    /// `global` does a system-wide font installation, such that every user on
    /// the system is able to access the fonts. This may require administrator
    /// permissions to modify system files, depending on the operating system.
    ///
    /// `local` does a user-local installation, and will make the font available
    /// for the current user only.
    #[arg(short, long, default_value = "local")]
    pub scope: Scope,
    /// Equivalent to `--scope global`.
    #[arg(short, long)]
    pub global: bool,
    /// Equivalent to `--scope local`.
    #[arg(short, long)]
    pub local: bool,
}
impl ScopeArgs {
    pub fn resolve(&self) -> Scope {
        if self.global {
            return Scope::Global;
        }
        if self.local {
            return Scope::Local;
        }
        self.scope
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Scope {
    Global,
    Local,
}

const STEADY_TICK_SPEED: std::time::Duration =
    std::time::Duration::from_millis(10);

pub fn run(args: InstallArgs) -> ExitCode {
    let mut cli = CliContext::new(&args.global);
    let mpb = cli.multiprogress();
    let cli = Arc::new(cli);
    let scope = args.scope.resolve();

    let config = tri_f!(::load_config, &cli);
    let sources = tri_f!(::create_sources, &cli, &config);

    let mut ok = true;
    for spec in &args.fonts {
        if let Some(source) = &spec.source {
            if !sources.is_enabled(source) {
                let _ = writeln!(
                    cli.error(),
                    "Source {} is not enabled (from {})",
                    source,
                    spec
                );
                ok = false;
            }
        }
    }
    if !ok {
        let _ = writeln!(cli.error(), "At least one font spec is invalid (see previous messages for details)");
        return ExitCode::FAILURE;
    }

    let source_ctx = tri_f!(::source_ctx, &cli, &config);
    let runtime = tri_f!(::tokio, &cli, &config);
    runtime.block_on(async {
        // 1. Get all possible resolutions for every single ID
        // TODO: Add progress bars

        // TODO: Fix the mess of allocation that this madness is
        //       It's not an important thing to do but it's just all-around ugly
        let resolved = sources.iter().map(|source| async {
            let specs = args.fonts.iter().enumerate().filter(|(_idx, spec)| {
                // A source only resolves those fonts that are either
                // specifically assigned to it or not assigned to any source in
                // particular
                match &spec.source {
                    None => true,
                    Some(id) if id == source.id() => true,
                    _ => false,
                }
            });

            let resolve = specs.map(|(idx, spec)| {
                source
                    .resolve_font(&source_ctx, spec)
                    .map_ok(move |resolved| (idx, resolved))
            });
            futures::future::try_join_all(resolve).await
        });
        let resolved_iter = futures::future::try_join_all(resolved)
            .await
            .map(|vecs| vecs.into_iter()
                .flatten());
        let resolved_iter = tri!(
            resolved_iter,
            cli.error(),
            "Could not resolve fonts"
        );

        // Vec<(font spec, Vec<resolved values>)>
        let mut specs: Vec<_> = args
            .fonts
            .into_iter()
            .map(|spec| (spec, vec![]))
            .collect();

        for (idx, resolved) in resolved_iter {
            specs[idx].1.extend(resolved);
        }

        // Ensure they're sorted for consistent results when running the code
        let mut errors = 0usize;
        for (spec, resolved) in &mut specs {
            if resolved.is_empty() {
                let _ = writeln!(cli.error(), "Font spec {} did not resolve to any fonts", spec);
                errors += 1;
                continue;
            }
            resolved.sort_by_key(|resolved| &resolved.reference);
        }
        if errors > 0 {
            let _ = writeln!(cli.error(), "{} error(s) occurred (see above)", errors);
            return ExitCode::FAILURE;
        }

        let _ = writeln!(cli.ok(), "Resolved fonts");

        // 2. Select exactly one resolution for each spec
        let specs = specs.into_iter()
            .map(|(spec, resolutions)| {
                // INVARIANT:
                // At this point there must be at least one element in each vec
                let resolution = if resolutions.len() > 1 {
                    let resolution = dialoguer::Select::new()
                        .with_prompt(format!("Select a resolution for {}", style(&spec).yellow()))
                        .default(0)
                        .items(resolutions.iter().map(|res| res.reference.to_string()))
                        .interact_on(&cli.term)
                        .context("handling user input")
                        .unwrap();
                    resolutions.into_iter().nth(resolution).unwrap()
                } else {
                    resolutions.into_iter().next().expect("invariant broken: at least one resolution must exist for each spec")
                };
                (spec, resolution)
            });

        // 3. Install each file

        let with_progress_bars = specs
            .map(|(spec, resolved)| {
                let pb = mpb.add(ProgressBar::new_spinner())
                    .with_message(format!("Downloading {}", style(&spec).yellow()));
                (spec, resolved, pb)
            });

        let pid = std::process::id();

        let future = with_progress_bars
            .enumerate()
            .map(async |(idx, (_spec, resolved, pb))| {
                for file in &resolved.files {
                    match &file.download {
                        Download::Url(url) => {
                            let response = source_ctx.http.get(url.clone())
                                .send()
                                .await
                                .context("getting download")?
                                .error_for_status()
                                .context("getting download")?;

                            // NOTE: We make the perhaps ill-fated assumption
                            // that whatever URL the source provides is a server
                            // that is not malicious for now
                            let expected_length = response.content_length();
                            let download = source_ctx.store.download_to_tmp(
                                response.bytes_stream(),
                                expected_length,
                                // TODO: Better way to ensure that temp files
                                //       don't use conflicting names
                                &format!("{}-{}-{}", pid, idx, file.name.file_name().unwrap_or("unnamed")),
                                cli.modify_files,
                                false,
                                |current| {
                                    pb.set_position(current as u64);
                                },
                            ).await?;
                            dbg!(download);
                        }
                    }
                }
                Ok::<_, anyhow::Error>(())
            });

        let future: anyhow::Result<_, _> = try_join_all(future).await;

        let future = tri!(future, cli.error(), "Failed to download fonts");

        ExitCode::SUCCESS
    })
}
