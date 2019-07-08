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
//! ClientAgent for information relevant to the User.
//!
//! * **Warp → ClientAgent**:
//! A new `ClientAgent` is created for each request.  The `ClientAgent` exists to manage concurrent
//! access to the (single) `Receiver`, which it can access behind an `Arc<Mutex>`.  The `ClientAgent`
//! polls the `Receiver` for any updates relevant to the current client.  If there are updates, the
//! `ClientAgent` filters them with the client's filters and passes any matching updates up to Warp.
//! The `ClientAgent` is also responsible for sending `subscribe` commands to Redis (via the
//! `Receiver`) when necessary.
//!
//! * **ClientAgent → Receiver**:
//! The Receiver receives data from Redis and stores it in a series of queues (one for each
//! ClientAgent). When (asynchronously) polled by the ClientAgent, it sends back the  messages
//! relevant to that ClientAgent and removes them from the queue.

pub mod config;
pub mod parse_client_request;
pub mod redis_to_client_stream;
