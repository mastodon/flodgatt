//! Streaming server for Mastodon
//!
//!
//! This server provides live, streaming updates for Mastodon clients.  Specifically, when a server
//! is running this sever, Mastodon clients can use either Server Sent Events or WebSockets to
//! connect to the server with the API described [in Mastodon's public API
//! documentation](https://docs.joinmastodon.org/api/streaming/).
//!
//! # Notes on data flow
//! * **Client Request → Warp**:
//! Warp filters for valid requests and parses request data. Based on that data, it generates a `User`
//! representing the client that made the request with data from the client's request and from
//! Postgres.  The `User` is authenticated, if appropriate.  Warp //! repeatedly polls the
//! StreamManager for information relevant to the User.
//!
//! * **Warp → StreamManager**:
//! A new `StreamManager` is created for each request.  The `StreamManager` exists to manage concurrent
//! access to the (single) `Receiver`, which it can access behind an `Arc<Mutex>`.  The `StreamManager`
//! polls the `Receiver` for any updates relevant to the current client.  If there are updates, the
//! `StreamManager` filters them with the client's filters and passes any matching updates up to Warp.
//! The `StreamManager` is also responsible for sending `subscribe` commands to Redis (via the
//! `Receiver`) when necessary.
//!
//! * **StreamManager → Receiver**:
//! The Receiver receives data from Redis and stores it in a series of queues (one for each
//! StreamManager). When (asynchronously) polled by the StreamManager, it sends back the  messages
//! relevant to that StreamManager and removes them from the queue.

pub mod config;
pub mod error;
pub mod postgres;
pub mod query;
pub mod receiver;
pub mod redis_cmd;
pub mod stream_manager;
pub mod timeline;
pub mod user;
pub mod ws;
