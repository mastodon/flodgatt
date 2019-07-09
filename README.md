# Mastodon Streaming Server
A WIP blazingly fast drop-in replacement for the Mastodon streaming api server.

## Current status
The streaming server is currently a work in progress.  However, it is now testable and, if
configured properly, would theoretically be usable in production—though production use is
not advisable until we have completed further testing. I would greatly appreciate any testing,
bug reports, or other feedback you could provide.

## Installation 
Installing the WIP version requires the Rust toolchain (the released version will be available
as a pre-compiled binary).  To install, clone this repository and run `cargo build` (to build
the server) `cargo run` (to both build and run the server), or `cargo build --release` (to
build the server with release optimizations).

## Connection to Mastodon
The streaming server expects to connect to a running development version of Mastodon built off of
the `master` branch.  Specifically, it needs to connect to both the Postgres database (to
authenticate users) and to the Redis database.  You should run Mastodon in whatever way you
normally do and configure the streaming server to connect to the appropriate databases.

## Configuring
You may edit the configuration variables in the `config.rs` module.  You can also overwrite the 
default config variables in the `.env` file.  Note that, by default, this server is configured
to run on port 4000.  This allows for easy testing with the development version of Mastodon
(which, by default, is configured to communicate with a streaming server running on
`localhost:4000`).  However, it also conflicts with the current/Node.js version of Mastodon's
streaming server, which runs on the same port.  Thus, to test this server, you should disable
the Node streaming server or move it to a non-conflicting port.

## Documentation
Build documentation with `cargo doc --open`, which will build the Markdown docs and open them
in your browser.  Please consult those docs for a detailed description of the code
structure/organization.  The documentation also contains additional notes about data flow and
options for configuration.

## Running 
As noted above, you can run the server with `cargo run`.  Alternatively, if you built the sever
using `cargo build` or `cargo build --release`, you can run the executable produced in the
`target/build/debug` folder or the `target/build/release` folder.

## Unit and (limited) integration tests
You can run basic unit test of the public Server Sent Event endpoints with `cargo test`.  You can
run integration tests of the authenticated SSE endpoints (which require a Postgres connection)
with `cargo test -- --ignored`.

## Manual testing
Once the streaming server is running, you can also test it manually.  You can test it using a
browser connected to the relevant Mastodon development server.  Or you can test the SSE endpoints
with `curl`, PostMan, or any other HTTP client.  Similarly, you can test the WebSocket endpoints
with `websocat` or any other WebSocket client.

## Memory/CPU usage
Note that memory usage is higher when running the development version of the streaming server (the
one generated with `cargo run` or `cargo build`).  If you are interested in measuring RAM or CPU
usage, you should likely run `cargo build --release` and test the release version of the executable.

## Load testing
I have not yet found a good way to test the streaming server under load.  I have experimented with
using `artillery` or other load-testing utilities.  However, every utility I am familiar with or
have found is built around either HTTP requests or WebSocket connections in which the client sends
messages.  I have not found a good solution to test receiving SSEs or WebSocket connections where
the client does not transmit data after establishing the connection.  If you are aware of a good
way to do load testing, please let me know.
