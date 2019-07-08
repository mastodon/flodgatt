pub mod client_agent;
pub mod receiver;
pub mod redis_cmd;

use crate::config;
pub use client_agent::ClientAgent;
use futures::{future::Future, stream::Stream, Async};
use std::{env, time};

pub fn send_updates_to_sse(
    mut client_agent: ClientAgent,
    connection: warp::sse::Sse,
) -> impl warp::reply::Reply {
    let sse_update_interval = env::var("SSE_UPDATE_INTERVAL")
        .map(|s| s.parse().expect("Valid config"))
        .unwrap_or(config::DEFAULT_SSE_UPDATE_INTERVAL);
    let event_stream = tokio::timer::Interval::new(
        time::Instant::now(),
        time::Duration::from_millis(sse_update_interval),
    )
    .filter_map(move |_| match client_agent.poll() {
        Ok(Async::Ready(Some(json_value))) => Some((
            warp::sse::event(json_value["event"].clone().to_string()),
            warp::sse::data(json_value["payload"].clone()),
        )),
        _ => None,
    });

    connection.reply(warp::sse::keep(event_stream, None))
}

/// Send a stream of replies to a WebSocket client
pub fn send_updates_to_ws(
    socket: warp::ws::WebSocket,
    mut stream: ClientAgent,
) -> impl futures::future::Future<Item = (), Error = ()> {
    let (ws_tx, mut ws_rx) = socket.split();

    // Create a pipe
    let (tx, rx) = futures::sync::mpsc::unbounded();

    // Send one end of it to a different thread and tell that end to forward whatever it gets
    // on to the websocket client
    warp::spawn(
        rx.map_err(|()| -> warp::Error { unreachable!() })
            .forward(ws_tx)
            .map_err(|_| ())
            .map(|_r| ()),
    );

    // For as long as the client is still connected, yeild a new event every 100 ms
    let ws_update_interval = env::var("WS_UPDATE_INTERVAL")
        .map(|s| s.parse().expect("Valid config"))
        .unwrap_or(config::DEFAULT_WS_UPDATE_INTERVAL);
    let event_stream = tokio::timer::Interval::new(
        time::Instant::now(),
        time::Duration::from_millis(ws_update_interval),
    )
    .take_while(move |_| match ws_rx.poll() {
        Ok(Async::Ready(None)) => futures::future::ok(false),
        _ => futures::future::ok(true),
    });

    // Every time you get an event from that stream, send it through the pipe
    event_stream
        .for_each(move |_json_value| {
            if let Ok(Async::Ready(Some(json_value))) = stream.poll() {
                let msg = warp::ws::Message::text(json_value.to_string());
                tx.unbounded_send(msg).expect("No send error");
            };
            Ok(())
        })
        .then(|msg| msg)
        .map_err(|e| println!("{}", e))
}
