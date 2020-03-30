use std::{error::Error, fmt};

pub fn die_with_msg(msg: impl fmt::Display) -> ! {
    eprintln!("FATAL ERROR: {}", msg);
    std::process::exit(1);
}

#[macro_export]
macro_rules! log_fatal {
    ($str:expr, $var:expr) => {{
        log::error!($str, $var);
        panic!();
    };};
}

#[derive(Debug)]
pub enum RedisParseErr {
    Incomplete,
    InvalidNumber(std::num::ParseIntError),
    NonNumericInput,
    InvalidLineStart(String),
    InvalidLineEnd,
    IncorrectRedisType,
    MissingField,
    UnsupportedTimeline,
    UnsupportedEvent(serde_json::Error),
}

impl fmt::Display for RedisParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", match self {
            Self::Incomplete => "The input from Redis does not form a complete message, likely because the input buffer filled partway through a message.  Save this input and try again with additional input from Redis.".to_string(),
            Self::InvalidNumber(e) => format!( "Redis input cannot be parsed: {}", e),
            Self::NonNumericInput => "Received non-numeric input when expecting a Redis number".to_string(),
            Self::InvalidLineStart(s) => format!("Got `{}` as a line start from Redis", s),
            Self::InvalidLineEnd => "Redis input ended before promised length".to_string(),
            Self::IncorrectRedisType => "Received a non-array when expecting a Redis array".to_string(),
            Self::MissingField => "Redis input was missing a required field".to_string(),
            Self::UnsupportedTimeline => "The raw timeline received from Redis could not be parsed into a supported timeline".to_string(),
            Self::UnsupportedEvent(e) => format!("The event text from Redis could not be parsed into a valid event: {}", e)
        })
    }
}

impl Error for RedisParseErr {}

impl From<std::num::ParseIntError> for RedisParseErr {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::InvalidNumber(error)
    }
}

impl From<serde_json::Error> for RedisParseErr {
    fn from(error: serde_json::Error) -> Self {
        Self::UnsupportedEvent(error)
    }
}

impl From<TimelineErr> for RedisParseErr {
    fn from(_: TimelineErr) -> Self {
        Self::UnsupportedTimeline
    }
}

#[derive(Debug)]
pub enum TimelineErr {
    RedisNamespaceMismatch,
    InvalidInput,
}

impl From<std::num::ParseIntError> for TimelineErr {
    fn from(_error: std::num::ParseIntError) -> Self {
        Self::InvalidInput
    }
}
