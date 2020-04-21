use super::super::{RedisConnErr, RedisParseErr};
use super::{Event, EventErr};
use crate::request::{Timeline, TimelineErr};

use std::fmt;
#[derive(Debug)]
pub enum Error {
    InvalidId,
    TimelineErr(TimelineErr),
    EventErr(EventErr),
    RedisParseErr(RedisParseErr),
    RedisConnErr(RedisConnErr),
    ChannelSendErr(tokio::sync::watch::error::SendError<(Timeline, Event)>),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use Error::*;
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

impl From<tokio::sync::watch::error::SendError<(Timeline, Event)>> for Error {
    fn from(error: tokio::sync::watch::error::SendError<(Timeline, Event)>) -> Self {
        Self::ChannelSendErr(error)
    }
}

impl From<EventErr> for Error {
    fn from(error: EventErr) -> Self {
        Self::EventErr(error)
    }
}

impl From<RedisConnErr> for Error {
    fn from(e: RedisConnErr) -> Self {
        Self::RedisConnErr(e)
    }
}

impl From<TimelineErr> for Error {
    fn from(e: TimelineErr) -> Self {
        Self::TimelineErr(e)
    }
}

impl From<RedisParseErr> for Error {
    fn from(e: RedisParseErr) -> Self {
        Self::RedisParseErr(e)
    }
}
