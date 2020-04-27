use crate::request;
use std::fmt;

#[derive(Debug)]
pub enum RedisConnErr {
    ConnectionErr { addr: String, inner: std::io::Error },
    InvalidRedisReply(String),
    UnknownRedisErr(std::io::Error),
    IncorrectPassword(String),
    MissingPassword,
    NotRedis(String),
    TimelineErr(request::TimelineErr),
}

impl RedisConnErr {
    pub(super) fn with_addr<T: AsRef<str>>(address: T, inner: std::io::Error) -> Self {
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
            InvalidRedisReply(unexpected_reply) => format!(
                "Received and unexpected reply from Redis: `{}`",
                unexpected_reply
            ),
            UnknownRedisErr(io_err) => {
                format!("Unexpected failure communicating with Redis: {}", io_err)
            }
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
            TimelineErr(inner) => format!("{}", inner),
        };
        write!(f, "{}", msg)
    }
}

impl From<request::TimelineErr> for RedisConnErr {
    fn from(e: request::TimelineErr) -> RedisConnErr {
        RedisConnErr::TimelineErr(e)
    }
}

impl From<std::io::Error> for RedisConnErr {
    fn from(e: std::io::Error) -> RedisConnErr {
        RedisConnErr::UnknownRedisErr(e)
    }
}
