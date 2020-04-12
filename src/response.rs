//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.

pub mod redis;
pub mod stream;

pub use redis::{Manager, ManagerErr};

#[cfg(feature = "bench")]
pub use redis::msg::{RedisMsg, RedisParseOutput};
