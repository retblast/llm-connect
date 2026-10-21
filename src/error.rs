use crate::config;
use crate::connection;
use crate::process;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum Error {
    Config(config::Error),
    Connection(connection::Error),
    Process(process::Error),
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Self::Config(error)
    }
}

impl From<connection::Error> for Error {
    fn from(error: connection::Error) -> Self {
        Self::Connection(error)
    }
}

impl From<process::Error> for Error {
    fn from(error: process::Error) -> Self {
        Self::Process(error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Config(error) => write!(f, "{error}"),
            Error::Connection(error) => write!(f, "{error}"),
            Error::Process(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Config(error) => std::error::Error::source(error),
            Error::Connection(error) => std::error::Error::source(error),
            Error::Process(error) => std::error::Error::source(error),
        }
    }
}
