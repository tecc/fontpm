use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Write};
use clap::{ArgMatches, Command};
use fontpm_api::collection;

pub mod refresh;
pub mod install;
mod config;

#[derive(Debug)]
pub enum Error {
    Generic(Box<dyn std::error::Error>),
    TODO(Option<String>),
    Custom(String),
    API(fontpm_api::Error)
}
impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use crate::commands::Error::*;
        match self {
            Generic(v) => Display::fmt(v, f),
            TODO(what) => match what {
                Some(v) => f.write_fmt(format_args!("TODO: {}", v)),
                None => f.write_str("TODO: This is not yet implemented.")
            },
            Custom(s) => f.write_str(s),
            API(e) => Display::fmt(e, f)
        }
    }
}

impl From<fontpm_api::Error> for Error {
    fn from(value: fontpm_api::Error) -> Self {
        Error::API(value)
    }
}

pub type Result = std::result::Result<Option<String>, Error>;
pub struct CommandAndRunner {
    pub description: Command,
    pub runner: fn (&ArgMatches) -> Result
}

pub fn all_commands() -> HashMap<String, CommandAndRunner> {
    return collection!{
        config::NAME => config::command(),
        refresh::NAME => refresh::command(),
        install::NAME => install::command()
    };
}