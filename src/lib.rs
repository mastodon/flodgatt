//! Streaming server for Mastodon
//!
//!
//! This server provides live, streaming updates for Mastodon clients.  Specifically, when a
//! server is running this sever, Mastodon clients can use either Server Sent Events or
//! WebSockets to connect to the server with the API described [in Mastodon's public API
//! documentation](https://docs.joinmastodon.org/api/streaming/).
//!
//! # Data Flow
//! * **Parsing the client request** When the client request first comes in, it is
//! parsed based on the endpoint it targets (for server sent events), its query parameters,
//! and its headers (for WebSocket).  Based on this data, we authenticate the user, retrieve
//! relevant user data from Postgres, and determine the timeline targeted by the request.
//! Successfully parsing the client request results in generating a `User` corresponding to
//! the request.  If any requests are invalid/not authorized, we reject them in this stage.
//! * **Streaming update from Redis to the client**: After the user request is parsed, we pass
//! the `User` data on to the `ClientAgent`.  The `ClientAgent` is responsible for
//! communicating the user's request to the `Receiver`, polling the `Receiver` for any
//! updates, and then for wording those updates on to the client.  The `Receiver`, in tern, is
//! responsible for managing the Redis subscriptions, periodically polling Redis, and sorting
//! the replies from Redis into queues for when it is polled by the `ClientAgent`.
//!
//! # Concurrency
//! The `Receiver` is created when the server is first initialized, and there is only one
//! `Receiver`.  Thus, the `Receiver` is a potential bottleneck.  On the other hand, each
//! client request results in a new green thread, which spawns its own `ClientAgent`.  Thus,
//! their will be many `ClientAgent`s polling a single `Receiver`.  Accordingly, it is very
//! important that polling the `Receiver` remain as fast as possible.  It is less important
//! that the `Receiver`'s poll of Redis be fast, since there will only ever be one
//! `Receiver`.
//!
//! # Configuration By default, the server uses config values from the `config.rs` module;
//! these values can be overwritten with environmental variables or in the `.env` file.  The
//! most important settings for performance control the frequency with which the `ClientAgent`
//! polls the `Receiver` and the frequency with which the `Receiver` polls Redis.
//!

//#![warn(clippy::pedantic)]
#![allow(clippy::try_err, clippy::match_bool)]

pub mod config;
pub mod err;
pub mod messages;
pub mod request;
pub mod response;
