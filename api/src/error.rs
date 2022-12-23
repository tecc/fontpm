use std::fmt::{Formatter, Write};

#[derive(Debug)]
pub enum Error {
    Generic(String),
    ConnectionError(String),
    IO(std::io::Error),
    Serialisation(String),
    Deserialisation(String)
}
impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            Generic(s) => f.write_str(s.as_str()),
            ConnectionError(s) => f.write_fmt(format_args!("Failed to connect: {}", s)),
            IO(e) => f.write_fmt(format_args!("Generic IO error: {}", e)),
            Serialisation(s) => f.write_fmt(format_args!("Serialisation error: {}", s)),
            Deserialisation(s) => f.write_fmt(format_args!("Deserialisation error: {}", s))
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(value)
    }
}

#[cfg(feature = "reqwest-util")]
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        // TODO: Expand?
        if err.is_connect() {
            Error::ConnectionError(format!("{}", err))
        } else {
            Error::Generic(format!("{}", err))
        }
    }
}
