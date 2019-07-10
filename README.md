Flóðgátt
========

[![Build Status](https://travis-ci.com/tootsuite/flodgatt.svg?branch=master)](https://travis-ci.com/tootsuite/flodgatt)

A blazingly fast drop-in replacement for the Mastodon streaming API server.

> **Current status:** This server is currently a **work in progress**. However, it is now testable and, if configured properly, would theoretically be usable in production—though production use is not advisable until we have completed further testing. I would greatly appreciate any testing, bug reports, or other feedback you could provide.

## Installation

Installing from source requires the Rust toolchain. Clone this repository and run `cargo build` (to build the server), or `cargo build --release` (to build the server with release optimizations).

### Configuring

The streaming server uses the same environment variables as the rest of Mastodon, which can either be passed to the process the standard way or through a `.env` file.

### Running

You can run the server with `cargo run`. Alternatively, if you built the sever using `cargo build` or `cargo build --release`, you can run the executable produced in the `target/build/debug` folder or the `target/build/release` folder.

## Documentation

Build documentation with `cargo doc --open`, which will build the Markdown docs and open them in your browser. Please consult those docs for a detailed description of the code structure/organization. The documentation also contains additional notes about data flow and options for configuration.

## Unit and (limited) integration tests

You can run basic unit test of the public Server Sent Event endpoints with `cargo test`. You can run integration tests of the authenticated SSE endpoints (which require a PostgreSQL connection) with `cargo test -- --ignored`.

### Manual testing

Once the streaming server is running, you can also test it manually. You can test it using a browser connected to the relevant Mastodon development server. Or you can test the SSE endpoints with `curl`, PostMan, or any other HTTP client. Similarly, you can test the WebSocket endpoints with `websocat` or any other WebSocket client.

### Memory/CPU usage

Note that memory usage is higher when running the development version of the streaming server (the one generated with `cargo run` or `cargo build`). If you are interested in measuring RAM or CPU usage, you should likely run `cargo build --release` and test the release version of the executable.

### Load testing

I have not yet found a good way to test the streaming server under load. I have experimented with using `artillery` or other load-testing utilities. However, every utility I am familiar with or have found is built around either HTTP requests or WebSocket connections in which the client sends messages. I have not found a good solution to test receiving SSEs or WebSocket connections where the client does not transmit data after establishing the connection. If you are aware of a good way to do load testing, please let me know.
