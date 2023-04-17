use clap::{arg, ArgAction, Command, command, Subcommand, value_parser, ValueEnum};
use fontpm_api::{info, ok, error};
use tokio::task::JoinSet;
use crate::commands::{CommandAndRunner, Error};
use crate::config::FpmConfig;
use crate::host_impl::FpmHostImpl;
use crate::runner;

pub const NAME: &str = "purge";

#[derive(ValueEnum, Clone)]
#[value(rename_all = "kebab-case")]
enum PurgeTarget {
    Cache,
    #[value(alias = "installed-fonts")]
    Fonts,
    All
}

const SUFFIX: &str = "This action is irrevocable. (Run this command with --confirm)";

async fn purge(target: PurgeTarget, config: FpmConfig) -> Result<String, Error> {
    match target {
        PurgeTarget::Cache => {
            let dir = config.cache_dir();
            if dir.exists() {
                info!("Purging cache...");
                tokio::fs::remove_dir_all(dir).await?;
                Ok("Purged cache.".to_string())
            } else {
                Ok("Cache already purged.".to_string())
            }
        },
        PurgeTarget::Fonts => {
            let dir = config.font_install_dir();
            if dir.exists() {
                info!("Purging installed fonts - you will need to reinstall any fonts you've already installed.");
                tokio::fs::remove_dir_all(dir).await?;
                Ok("Purged installed fonts. You will need to reinstall all fonts you've already installed.".to_string())
            } else {
                Ok("All installed fonts have already been deleted.".to_string())
            }
        },
        PurgeTarget::All => {
            panic!("purge can not be called with target All");
        }
    }
}

runner! { args =>
    let confirm = args.get_flag("confirm");
    let target: &PurgeTarget = args.get_one("target").unwrap();
    if !confirm {
        return Err(Error::ConfirmationNeeded(format!("{} {}", match target {
            PurgeTarget::Cache => "Are you sure you want to purge the cache?",
            PurgeTarget::Fonts => "Are you sure you want to purge all FontPM-installed fonts?",
            PurgeTarget::All => "Are you sure you want to purge all FontPM data?"
        }, SUFFIX)))
    }
    let config = FpmConfig::load()?;
    let mut tasks = JoinSet::new();
    match target {
        PurgeTarget::All => {
            tasks.spawn(purge(PurgeTarget::Cache, config.clone()));
            tasks.spawn(purge(PurgeTarget::Cache, config));
        },
        other => {
            tasks.spawn(purge(other.clone(), config));
        }
    }

    let mut command_result = Ok(None);
    while let Some(result) = tasks.join_next().await {
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                error!("Could not join task: {}", e);
                continue
            }
        };
        match result {
            Ok(message) => {
                ok!("{}", message);
            },
            Err(e) => {
                command_result = Err(e);
            }
        }
    }

    command_result
}

pub fn command() -> CommandAndRunner {
    let command = Command::new(NAME)
        .about("Purges all of FontPM's cached files.")
        .args(vec![
            arg!(<target> "What to purge.")
                .value_parser(value_parser!(PurgeTarget)),
            arg!(-c --confirm "Confirms that you want to.")
                .action(ArgAction::SetTrue)
        ]);
    return CommandAndRunner {
        description: command,
        runner: Box::new(runner)
    };
}