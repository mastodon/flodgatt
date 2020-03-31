//use std::{error::Error, fmt};

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
