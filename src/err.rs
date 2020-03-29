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

#[derive(Debug, Clone, PartialEq)]
pub enum RedisParseErr {
    Incomplete,
    InvalidNumber(std::num::ParseIntError),
    NonNumericInput,
    InvalidLineStart(String),
    IncorrectRedisType,
}

impl fmt::Display for RedisParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", match self {
            Self::Incomplete => "The input from Redis does not form a complete message, likely because the input buffer filled partway through a message.  Save this input and try again with additional input from Redis.".to_string(),
            Self::InvalidNumber(e) => format!( "Redis input cannot be parsed: {}", e),
            _ => "TODO".to_string(),
        })
    }
}

impl Error for RedisParseErr {}

impl From<std::num::ParseIntError> for RedisParseErr {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::InvalidNumber(error)
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
