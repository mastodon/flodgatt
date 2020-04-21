//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.

pub use event::Event;
pub use redis::Manager as RedisManager;
pub use stream::{Sse as SseStream, Ws as WsStream};

pub(self) use event::err::Event as EventErr;
pub(self) use event::Payload;

pub(crate) mod event;
mod redis;
mod stream;

pub use redis::Error;

#[cfg(feature = "bench")]
pub use redis::msg::{RedisMsg, RedisParseOutput};
