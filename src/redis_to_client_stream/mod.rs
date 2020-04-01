//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.
mod client_agent;
mod event_stream;
mod receiver;
mod redis;

pub use {client_agent::ClientAgent, event_stream::EventStream};

// TODO remove
pub use redis::redis_msg;

//#[cfg(test)]
//pub use receiver::process_messages;
//#[cfg(test)]
pub use receiver::{MessageQueues, MsgQueue, Receiver, ReceiverErr};
//#[cfg(test)]
//pub use redis::redis_msg::{RedisMsg, RedisUtf8};
