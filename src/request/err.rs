use std::fmt;
#[derive(Debug)]
pub enum Error {
    PgPool(r2d2::Error),
    Pg(postgres::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use Error::*;
        let msg = match self {
            PgPool(e) => format!("{}", e),
            Pg(e) => format!("{}", e),
        };
        write!(f, "{}", msg)
    }
}

impl From<r2d2::Error> for Error {
    fn from(e: r2d2::Error) -> Self {
        Self::PgPool(e)
    }
}
impl From<postgres::Error> for Error {
    fn from(e: postgres::Error) -> Self {
        Self::Pg(e)
    }
}
// TODO make Timeline & TimelineErr their own top-level module
#[derive(Debug)]
pub enum Timeline {
    MissingHashtag,
    InvalidInput,
    BadTag,
}

impl std::error::Error for Timeline {}

impl From<std::num::ParseIntError> for Timeline {
    fn from(_error: std::num::ParseIntError) -> Self {
        Self::InvalidInput
    }
}

impl fmt::Display for Timeline {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use Timeline::*;
        let msg = match self {
            InvalidInput => "The timeline text from Redis could not be parsed into a supported timeline.  TODO: add incoming timeline text",
            MissingHashtag => "Attempted to send a hashtag timeline without supplying a tag name",
            BadTag => "No hashtag exists with the specified hashtag ID"
        };
        write!(f, "{}", msg)
    }
}
