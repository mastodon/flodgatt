use super::super::{redis::RedisConnErr, redis_msg::RedisParseErr};
use crate::err::TimelineErr;

use serde_json;

#[derive(Debug)]
pub enum ReceiverErr {
    TimelineErr(TimelineErr),
    EventErr(serde_json::Error),
    RedisParseErr(RedisParseErr),
    RedisConnErr(RedisConnErr),
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
