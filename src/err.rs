use crate::config;
use crate::request;
use crate::response;

use std::fmt;

pub enum Error {
    Response(response::Error),
    Logger(log::SetLoggerError),
    Postgres(request::Error),
    Unrecoverable,
    StdIo(std::io::Error),
    Config(config::Error),
}

impl Error {
    pub fn log(msg: impl fmt::Display) {
        eprintln!("Error: {}", msg);
    }
}

impl std::error::Error for Error {}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use Error::*;
        write!(
            f,
            "{}",
            match self {
                Response(e) => format!("{}", e),
                Logger(e) => format!("{}", e),
                StdIo(e) => format!("{}", e),
                Postgres(e) => format!("could not connect to Postgres.\n{:7}{}", "", e),
                Config(e) => format!("{}", e),
                Unrecoverable => "Flodgatt will now shut down.".into(),
            }
        )
    }
}

#[doc(hidden)]
impl From<request::Error> for Error {
    fn from(e: request::Error) -> Self {
        Self::Postgres(e)
    }
}

#[doc(hidden)]
impl From<response::Error> for Error {
    fn from(e: response::Error) -> Self {
        Self::Response(e)
    }
}

#[doc(hidden)]
impl From<config::Error> for Error {
    fn from(e: config::Error) -> Self {
        Self::Config(e)
    }
}

#[doc(hidden)]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::StdIo(e)
    }
}

#[doc(hidden)]
impl From<log::SetLoggerError> for Error {
    fn from(e: log::SetLoggerError) -> Self {
        Self::Logger(e)
    }
}
