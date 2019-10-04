//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.
pub mod client_agent;
pub mod receiver;
pub mod redis_cmd;
pub mod redis_stream;

use crate::config;
pub use client_agent::ClientAgent;
use futures::{future::Future, stream::Stream, Async};
use log;
use std::time;

/// Send a stream of replies to a Server Sent Events client.
pub fn send_updates_to_sse(
    mut client_agent: ClientAgent,
    connection: warp::sse::Sse,
    update_interval: time::Duration,
) -> impl warp::reply::Reply {
    let event_stream = tokio::timer::Interval::new(time::Instant::now(), update_interval)
        .filter_map(move |_| match client_agent.poll() {
            Ok(Async::Ready(Some(json_value))) => Some((
                warp::sse::event(json_value["event"].clone().to_string()),
                warp::sse::data(json_value["payload"].clone()),
            )),
            _ => None,
        });

    connection.reply(warp::sse::keep(event_stream, None))
}

/// Send a stream of replies to a WebSocket client.
pub fn send_updates_to_ws(
    socket: warp::ws::WebSocket,
    mut stream: ClientAgent,
    update_interval: time::Duration,
) -> impl futures::future::Future<Item = (), Error = ()> {
    let (ws_tx, mut ws_rx) = socket.split();

    // Create a pipe
    let (tx, rx) = futures::sync::mpsc::unbounded();

    // Send one end of it to a different thread and tell that end to forward whatever it gets
    // on to the websocket client
    warp::spawn(
        rx.map_err(|()| -> warp::Error { unreachable!() })
            .forward(ws_tx)
            .map(|_r| ())
            .map_err(|e| eprintln!("websocket send error: {}", e)),
    );

    // Yield new events for as long as the client is still connected
    let event_stream = tokio::timer::Interval::new(time::Instant::now(), update_interval)
        .take_while(move |_| match ws_rx.poll() {
            Ok(Async::NotReady) | Ok(Async::Ready(Some(_))) => futures::future::ok(true),
            Ok(Async::Ready(None)) => {
                // TODO: consider whether we should manually drop closed connections here
                log::info!("Client closed WebSocket connection");
                futures::future::ok(false)
            }
            Err(e) => {
                log::warn!("{}", e);
                futures::future::ok(false)
            }
        });

    let mut time = time::Instant::now();

    // Every time you get an event from that stream, send it through the pipe
    event_stream
        .for_each(move |_instant| {
            if let Ok(Async::Ready(Some(json_value))) = stream.poll() {
                let msg = warp::ws::Message::text(json_value.to_string());
                tx.unbounded_send(msg).expect("No send error");
            };
            if time.elapsed() > time::Duration::from_secs(30) {
                let msg = warp::ws::Message::ping(Vec::new());
                tx.unbounded_send(msg).expect("Can ping");
                println!("Sent empty ping");
                time = time::Instant::now();
            }
            Ok(())
        })
        .then(move |result| {
            // TODO: consider whether we should manually drop closed connections here
            log::info!("WebSocket connection closed.");
            result
        })
        .map_err(move |e| log::error!("{}", e))
}
