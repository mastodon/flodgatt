//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.

pub mod redis;
pub mod stream;

pub(crate) use redis::ManagerErr;

#[cfg(feature = "bench")]
pub use redis::msg::{RedisMsg, RedisParseOutput};
