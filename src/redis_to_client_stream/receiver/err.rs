use super::super::redis::{RedisConnErr, RedisParseErr};
use crate::err::TimelineErr;
use crate::messages::Event;
use crate::parse_client_request::Timeline;

use serde_json;
use std::fmt;

#[derive(Debug)]
pub enum ReceiverErr {
    InvalidId,
    TimelineErr(TimelineErr),
    EventErr(serde_json::Error),
    RedisParseErr(RedisParseErr),
    RedisConnErr(RedisConnErr),
    ChannelSendErr(tokio::sync::watch::error::SendError<(Timeline, Event)>),
}

impl std::error::Error for ReceiverErr {}

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
            ChannelSendErr(inner) => write!(f, "{}", inner),
        }?;
        Ok(())
    }
}
impl From<tokio::sync::watch::error::SendError<(Timeline, Event)>> for ReceiverErr {
    fn from(error: tokio::sync::watch::error::SendError<(Timeline, Event)>) -> Self {
        Self::ChannelSendErr(error)
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
