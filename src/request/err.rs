use std::fmt;
#[derive(Debug)]
pub enum RequestErr {
    Unknown,
    PgPool(r2d2::Error),
}

impl std::error::Error for RequestErr {}

impl fmt::Display for RequestErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use RequestErr::*;
        let msg = match self {
            Unknown => "Encountered an unrecoverable error related to handling a request".into(),
            PgPool(e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl From<r2d2::Error> for RequestErr {
    fn from(e: r2d2::Error) -> Self {
        Self::PgPool(e)
    }
}
