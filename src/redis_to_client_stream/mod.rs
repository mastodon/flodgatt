//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.
mod client_agent;
mod receiver;
mod redis;
mod event_stream;

pub use {client_agent::ClientAgent, event_stream::EventStream};



