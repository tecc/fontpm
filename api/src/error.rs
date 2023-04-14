use std::fmt::{Formatter, Write};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),
    #[error("connection error: {0}")]
    ConnectionError(String),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("serialisation error: {0}")]
    Serialisation(String),
    #[error("deserialisation error: {0}")]
    Deserialisation(String),
    #[error("no such font family: {0}")]
    NoSuchFamily(String),
}
pub type Result<T> = std::result::Result<T, Error>;

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
