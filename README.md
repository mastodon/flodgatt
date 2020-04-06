Flóðgátt
========

[![Build Status](https://travis-ci.com/tootsuite/flodgatt.svg?branch=master)](https://travis-ci.com/tootsuite/flodgatt)

A blazingly fast drop-in replacement for the Mastodon streaming API server.

> **Current status:** This server is currently a **work in progress**. However, it is now testable
> and, if configured properly, would theoretically be usable in production—though production use
> is not advisable until we have completed further testing. I would greatly appreciate any
> testing, bug reports, or other feedback you could provide.

## Installation

Starting from version 0.3, Flóðgátt can be installed for Linux by installing the pre-built
binaries released on GitHub.  Simply download the binary (extracting it if necessary), set it to
executable (`chmod +x`) and run it.  Note that you will likely need to configure the Postgres
connection before you can successfully connect.

### Configuration Examples

If you are running Mastodon with its [standard Development
settings](https://docs.joinmastodon.org/dev/setup/), then you should be able to run `flodgatt`
without any configuration.  (You will, of course, need to ensure that the Node streaming server is
not running at the same time as Flodgatt.  If you normally run the development servers with
`foreman start`, you should edit the `Procfile.dev` file to remove the line that starts the Node
server.  To run `flodgatt` with a production instance of Mastodon, you should ensure that the
`mastodon-streaming` systemd service is not running.)

You will likely wish to use the environmental variable `RUST_LOG=warn` to enable debugging warnings.

If you are running Mastodon with its standard Production settings and connect to Postgres with the
Ident authentication method, then you can use the following procedure to launch Flodgatt.
 * Change to the user that satisfies the Ident requirement (typically "mastodon" with default
   settints).  For example: `su mastodon`
 * Use environmental variables to set the user, database, and host names.  For example:
   `DB_NAME="mastodon_production" DB_USER="mastodon" DB_HOST="/var/run/postgresql" RUST_LOG=warn
   flodgatt`
 
If you have any difficulty connecting, note that, if run with `RUST_LOG=warn` Flodgatt will print
both the environmental variables it received and the parsed configuration variables it generated
from those environmental variables.  You can use this info to debug the connection.

Flóðgátt is tested against the [default Mastodon nginx config](https://github.com/tootsuite/mastodon/blob/master/dist/nginx.conf) and treats that as the known-good configuration.

### Advanced Configuration

The streaming server will eventually use the same environment variables as the rest of Mastodon,
and currently uses a subset of those variables.  Supported variables are listed in
`/src/config.rs`.  Supported environmental variables are either passed to Flóðgátt at runtime or
through a `.env` file.

Note that the default values for the `postgres` connection do not correspond to those typically
used in production.  Thus, you will need to configure the connection either env vars or a `.env`
file if you intend to connect Flóðgátt to a production database.

If you set the `SOCKET` environmental variable, you must set the nginx `proxy_pass` variable to
the same socket (with the file prefixed by `http://unix:`).

Additionally, note that connecting Flóðgátt to Postgres with the `ident` method requires running
Flóðgátt as the user who owns the mastodon database (typically `mastodon`).

## Building from source

Installing from source requires the Rust toolchain. Clone this repository and run `cargo build`
(to build the server), or `cargo build --release` (to build the server with release
optimizations).

### Running the built server

You can run the server with `cargo run`. Alternatively, if you built the sever using `cargo build`
or `cargo build --release`, you can run the executable produced in the `target/build/debug` folder
or the `target/build/release` folder.

### Building documentation 

Build documentation with `cargo doc --open`, which will build the Markdown docs and open them in
your browser. Please consult those docs for a detailed description of the code
structure/organization. The documentation also contains additional notes about data flow and
options for configuration.

### Testing

You can run basic unit tests with `cargo test`.

### Manual testing

Once the streaming server is running, you can also test it manually. You can test it using a
browser connected to the relevant Mastodon development server. Or you can test the SSE endpoints
with `curl`, PostMan, or any other HTTP client. Similarly, you can test the WebSocket endpoints
with `websocat` or any other WebSocket client.

### Memory/CPU usage

Note that memory usage is higher when running the development version of the streaming server (the
one generated with `cargo run` or `cargo build`). If you are interested in measuring RAM or CPU
usage, you should likely run `cargo build --release` and test the release version of the
executable.

### Load testing

I have not yet found a good way to test the streaming server under load. I have experimented with
using `artillery` or other load-testing utilities. However, every utility I am familiar with or
have found is built around either HTTP requests or WebSocket connections in which the client sends
messages. I have not found a good solution to test receiving SSEs or WebSocket connections where
the client does not transmit data after establishing the connection. If you are aware of a good
way to do load testing, please let me know.


## Contributing

Issues and pull requests are welcome. Flóðgátt is governed by the same Code of Conduct as Mastodon
as a whole.
