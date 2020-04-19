//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.

pub use crate::event::Event;
pub use redis::Manager as RedisManager;
pub use stream::{Sse as SseStream, Ws as WsStream};

mod redis;
mod stream;

pub use redis::Error;

#[cfg(feature = "bench")]
pub use redis::msg::{RedisMsg, RedisParseOutput};
