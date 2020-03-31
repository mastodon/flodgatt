use std::fmt;

#[derive(Debug)]
pub enum RedisConnErr {
    ConnectionErr { addr: String, inner: std::io::Error },
    // TODO ^^^^ better name?
    UnknownRedisErr(String),
    IncorrectPassword(String),
    MissingPassword,
    NotRedis(String),
}

impl RedisConnErr {
    pub fn with_addr<T: AsRef<str>>(address: T, inner: std::io::Error) -> Self {
        Self::ConnectionErr {
            addr: address.as_ref().to_string(),
            inner,
        }
    }
}

impl fmt::Display for RedisConnErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use RedisConnErr::*;
        let msg = match self {
            ConnectionErr { addr, inner } => format!(
                "Error connecting to Redis at {}.\n\
                 Connection Error: {}",
                addr, inner
            ),
            UnknownRedisErr(unexpected_reply) => format!(
                "Could not connect to Redis for an unknown reason.  Expected `+PONG` reply but got `{}`",
                unexpected_reply
            ),
            IncorrectPassword(attempted_password) => format!(
                "Incorrect Redis password.  You supplied `{}`.\n \
                 Please supply correct password with REDIS_PASSWORD environmental variable.",
                attempted_password
            ),
            MissingPassword => "Invalid authentication for Redis.  Redis is configured to require \
                                a password, but you did not provide one. \n\
                                Set a password using the REDIS_PASSWORD environmental variable."
                .to_string(),
            NotRedis(addr) => format!(
                "The server at {} is not a Redis server.  Please update the REDIS_HOST and/or \
                 REDIS_PORT environmental variables and try again.",
                addr
            ),
        };
        write!(f, "{}", msg)
    }
}

// die_with_msg(format!(
//           r"Incorrect Redis password.  You supplied `{}`.
//            Please supply correct password with REDIS_PASSWORD environmental variable.",
//           password,
//       ))

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
