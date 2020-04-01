//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.
mod client_agent;
mod event_stream;
mod receiver;
mod redis;

pub use {client_agent::ClientAgent, event_stream::EventStream, receiver::Receiver};
