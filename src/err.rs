use crate::request;
use crate::response;
use std::fmt;

pub enum Error {
    ReceiverErr(response::Error),
    Logger(log::SetLoggerError),
    Postgres(request::Error),
    Unrecoverable,
    StdIo(std::io::Error),
    // config errs
    UrlParse(url::ParseError),
    UrlEncoding(urlencoding::FromUrlEncodingError),
    ConfigErr(String),
}

impl Error {
    pub fn log(msg: impl fmt::Display) {
        eprintln!("{}", msg);
    }

    pub fn config<T: fmt::Display>(var: T, value: T, allowed_vals: T) -> Self {
        Self::ConfigErr(format!(
            "{0} is set to `{1}`, which is invalid.\n{3:7}{0} must be {2}.",
            var, value, allowed_vals, ""
        ))
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
                ReceiverErr(e) => format!("{}", e),
                Logger(e) => format!("{}", e),
                StdIo(e) => format!("{}", e),
                Postgres(e) => format!("could not connect to Postgres.\n{:7}{}", "", e),
                ConfigErr(e) => e.to_string(),
                UrlParse(e) => format!("could parse Postgres URL.\n{:7}{}", "", e),
                UrlEncoding(e) => format!("could not parse POSTGRES_URL.\n{:7}{:?}", "", e),
                Unrecoverable => "Flodgatt will now shut down.".into(),
            }
        )
    }
}

impl From<request::Error> for Error {
    fn from(e: request::Error) -> Self {
        Self::Postgres(e)
    }
}

impl From<response::Error> for Error {
    fn from(e: response::Error) -> Self {
        Self::ReceiverErr(e)
    }
}
impl From<urlencoding::FromUrlEncodingError> for Error {
    fn from(e: urlencoding::FromUrlEncodingError) -> Self {
        Self::UrlEncoding(e)
    }
}
impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::UrlParse(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::StdIo(e)
    }
}
impl From<log::SetLoggerError> for Error {
    fn from(e: log::SetLoggerError) -> Self {
        Self::Logger(e)
    }
}
