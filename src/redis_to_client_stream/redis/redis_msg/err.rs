use std::{error::Error, fmt};

#[derive(Debug)]
pub enum RedisParseErr {
    Incomplete,
    InvalidNumber(std::num::ParseIntError),
    InvalidLineStart(String),
    InvalidLineEnd,
    IncorrectRedisType,
    MissingField,
}

impl fmt::Display for RedisParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use RedisParseErr::*;
        let msg = match self {
            Incomplete => "The input from Redis does not form a complete message, likely because \
                           the input buffer filled partway through a message.  Save this input \
                           and try again with additional input from Redis."
                .to_string(),
            InvalidNumber(parse_int_err) => format!(
                "Redis indicated that an item would be a number, but it could not be parsed: {}",
                parse_int_err
            ),

            InvalidLineStart(line_start_char) => format!(
                "A line from Redis started with `{}`, which is not a valid character to indicate \
                 the type of the Redis line.",
                line_start_char
            ),
            InvalidLineEnd => "A Redis line ended before expected line length".to_string(),
            IncorrectRedisType => "Received a Redis type that is not supported in this context.  \
                                   Flodgatt expects each message from Redis to be a Redis array \
                                   consisting of bulk strings or integers."
                .to_string(),
            MissingField => "Redis input was missing a field Flodgatt expected (e.g., a `message` \
                             without a payload line)"
                .to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl Error for RedisParseErr {}

impl From<std::num::ParseIntError> for RedisParseErr {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::InvalidNumber(error)
    }
}
