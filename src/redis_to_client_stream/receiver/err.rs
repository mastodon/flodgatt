use super::super::redis::{RedisConnErr, RedisParseErr};
use crate::err::TimelineErr;

use serde_json;
use std::fmt;

#[derive(Debug)]
pub enum ReceiverErr {
    InvalidId,
    TimelineErr(TimelineErr),
    EventErr(serde_json::Error),
    RedisParseErr(RedisParseErr),
    RedisConnErr(RedisConnErr),
}

impl fmt::Display for ReceiverErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use ReceiverErr::*;
        match self {
            InvalidId => write!(
                f,
                "Attempted to get messages for a subscription that had not been set up."
            ),
            EventErr(inner) => write!(f, "{}", inner),
            RedisParseErr(inner) => write!(f, "{}", inner),
            RedisConnErr(inner) => write!(f, "{}", inner),
            TimelineErr(inner) => write!(f, "{}", inner),
        }?;
        Ok(())
    }
}

impl From<serde_json::Error> for ReceiverErr {
    fn from(error: serde_json::Error) -> Self {
        Self::EventErr(error)
    }
}

impl From<RedisConnErr> for ReceiverErr {
    fn from(e: RedisConnErr) -> Self {
        Self::RedisConnErr(e)
    }
}

impl From<TimelineErr> for ReceiverErr {
    fn from(e: TimelineErr) -> Self {
        Self::TimelineErr(e)
    }
}

impl From<RedisParseErr> for ReceiverErr {
    fn from(e: RedisParseErr) -> Self {
        Self::RedisParseErr(e)
    }
}
