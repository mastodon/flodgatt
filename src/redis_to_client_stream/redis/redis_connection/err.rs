use super::super::redis_msg::RedisParseErr;
use crate::err::TimelineErr;

#[derive(Debug)]
pub enum RedisConnErr {
    TimelineErr(TimelineErr),
    EventErr(serde_json::Error),
    RedisParseErr(RedisParseErr),
}

impl From<serde_json::Error> for RedisConnErr {
    fn from(error: serde_json::Error) -> Self {
        Self::EventErr(error)
    }
}

impl From<TimelineErr> for RedisConnErr {
    fn from(e: TimelineErr) -> Self {
        Self::TimelineErr(e)
    }
}

impl From<RedisParseErr> for RedisConnErr {
    fn from(e: RedisParseErr) -> Self {
        Self::RedisParseErr(e)
    }
}

// impl fmt::Display for RedisParseErr {
//     fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
//         use RedisParseErr::*;
//         let msg = match self {
//             Incomplete => "The input from Redis does not form a complete message, likely because \
//                            the input buffer filled partway through a message.  Save this input \
//                            and try again with additional input from Redis."
//                 .to_string(),
//             InvalidNumber(parse_int_err) => format!(
//                 "Redis indicated that an item would be a number, but it could not be parsed: {}",
//                 parse_int_err
//             ),

//             InvalidLineStart(line_start_char) => format!(
//                 "A line from Redis started with `{}`, which is not a valid character to indicate \
//                  the type of the Redis line.",
//                 line_start_char
//             ),
//             InvalidLineEnd => "A Redis line ended before expected line length".to_string(),
//             IncorrectRedisType => "Received a Redis type that is not supported in this context.  \
//                                    Flodgatt expects each message from Redis to be a Redis array \
//                                    consisting of bulk strings or integers."
//                 .to_string(),
//             MissingField => "Redis input was missing a field Flodgatt expected (e.g., a `message` \
//                              without a payload line)"
//                 .to_string(),
//             UnsupportedTimeline => {
//                 "The raw timeline received from Redis could not be parsed into a \
//                 supported timeline"
//                     .to_string()
//             }
//             UnsupportedEvent(e) => format!(
//                 "The event text from Redis could not be parsed into a valid event: {}",
//                 e
//             ),
//         };
//         write!(f, "{}", msg)
//     }
// }
