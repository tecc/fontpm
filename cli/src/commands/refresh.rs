use clap::{ArgMatches, Command};
use fontpm_api::{debug, error, Error as FError, info, Source};
use fontpm_api::source::RefreshOutput;
use crate::commands::{self, Error, CommandAndRunner};
use crate::host_impl::FpmHostImpl;
use crate::sources::create_enabled_sources;

pub const NAME: &str = "refresh";

fn runner(args: &ArgMatches) -> commands::Result {
    let host = FpmHostImpl::create(None)?;
    let sources = create_enabled_sources(Some(&host))?;

    {
        let source_display_names: Vec<String> = sources.iter()
            .map(|v| v.name().to_string())
            .collect();
        info!("Refreshing: {}", source_display_names.join(", "));
    }

    let results = futures::executor::block_on(futures::future::join_all(
        sources.iter().map(|source| async {
            let result = source.refresh().await;
            match result {
                Ok(v) => {
                    debug!("[{}] Successfully refreshed{}", source.name(), if v == RefreshOutput::AlreadyUpToDate {
                        " (already up-to-date)"
                    } else {
                        ""
                    });
                    Ok(v)
                },
                Err(e) => {
                    error!("[{}] Error when refreshing: {}", source.name(), e);
                    Err(e)
                }
            }
        })
    ));

    let mut errored = 0;
    let mut downloaded = 0;
    let mut already_up_to_date = 0;
    for result in results {
        match result {
            Ok(v) => match v {
                RefreshOutput::Downloaded => downloaded += 1,
                RefreshOutput::AlreadyUpToDate => already_up_to_date += 1
            },
            Err(_) => errored += 1,
        }
    }

    fn plural(v: i32) -> &'static str {
        if v > 1 {
            "s"
        }  else {
            ""
        }
    }
    if errored > 0 {
        Err(Error::Custom(format!("{} source{} failed to refresh", errored, plural(errored))))
    } else {
        Ok(Some(format!(
            "{}{}", if downloaded > 0 {
                format!("{} source{} refreshed", downloaded, plural(downloaded))
            } else {
                "".to_string()
            },
            if already_up_to_date > 0 {
                format!("{} source{} already up-to-date", already_up_to_date, plural(already_up_to_date))
            } else {
                format!("")
            }
        )))
    }
}

pub fn command() -> CommandAndRunner {
    return CommandAndRunner {
        description: Command::new(NAME)
            .about("Refresh the local index of all available fonts."),
        runner
    };
}