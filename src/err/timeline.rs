use std::fmt;

#[derive(Debug)]
pub enum TimelineErr {
    MissingHashtag,
    InvalidInput,
}

impl std::error::Error for TimelineErr {}

impl From<std::num::ParseIntError> for TimelineErr {
    fn from(_error: std::num::ParseIntError) -> Self {
        Self::InvalidInput
    }
}

impl fmt::Display for TimelineErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use TimelineErr::*;
        let msg = match self {
            InvalidInput => "The timeline text from Redis could not be parsed into a supported timeline.  TODO: add incoming timeline text",
            MissingHashtag => "Attempted to send a hashtag timeline without supplying a tag name",
        };
        write!(f, "{}", msg)
    }
}
