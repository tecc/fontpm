use std::collections::HashMap;
use std::fmt::Debug;
use clap::{ArgMatches, Command};
use fontpm_api::collection;
use crate::generate;

mod refresh;
mod install;
mod config;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("TODO: {}", if let Some(m) = .0 { m.as_ref() } else { "This is not yet implemented." })]
    TODO(Option<String>),
    #[error("{0}")]
    Custom(String),
    #[error("{0}")]
    API(#[from] fontpm_api::Error),
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("generation failed: {0}")]
    Generate(#[from] generate::GenerateError),
    #[error("invalid argument(s): {0}")]
    ArgMatch(#[from] clap::Error)

}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::API(fontpm_api::Error::IO(value))
    }
}

pub type Result = std::result::Result<Option<String>, Error>;
pub struct CommandAndRunner {
    pub description: Command,
    pub runner: Box<dyn Runner>
}
#[async_trait]
pub trait Runner {
    async fn run(&self, matches: &ArgMatches) -> Result;
}
#[macro_export] macro_rules! runner {
    {$name:tt: $args:tt => $($tokens:tt)*} => {
        #[allow(non_camel_case_types)]
        struct $name;
        #[async_trait]
        impl $crate::commands::Runner for $name {
            async fn run(&self, $args: &::clap::ArgMatches) -> $crate::commands::Result {
                #[allow(unused_imports)]
                use ::clap::FromArgMatches;
                $($tokens)*
            }
        }
    };
    {$args:tt => $($tokens:tt)*} => {
        runner!{runner: $args => $($tokens)*}
    }
}

pub fn all_commands() -> HashMap<String, CommandAndRunner> {
    return collection!{
        config::NAME => config::command(),
        refresh::NAME => refresh::command(),
        install::NAME => install::command()
    };
}