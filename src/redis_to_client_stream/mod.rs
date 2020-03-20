//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.
pub mod client_agent;
pub mod message;
pub mod receiver;
pub mod redis;
pub use client_agent::ClientAgent;
use futures::{future::Future, stream::Stream, Async};
use log;
use serde_json::json;
use std::time;

/// Send a stream of replies to a Server Sent Events client.
pub fn send_updates_to_sse(
    mut client_agent: ClientAgent,
    connection: warp::sse::Sse,
    update_interval: time::Duration,
) -> impl warp::reply::Reply {
    let event_stream = tokio::timer::Interval::new(time::Instant::now(), update_interval)
        .filter_map(move |_| match client_agent.poll() {
            Ok(Async::Ready(Some(msg))) => Some((
                warp::sse::event(msg.event()),
                warp::sse::data(msg.payload()),
            )),
            _ => None,
        });

    connection.reply(
        warp::sse::keep_alive()
            .interval(time::Duration::from_secs(30))
            .text("thump".to_string())
            .stream(event_stream),
    )
}

/// Send a stream of replies to a WebSocket client.
pub fn send_updates_to_ws(
    socket: warp::ws::WebSocket,
    mut client_agent: ClientAgent,
    update_interval: time::Duration,
) -> impl futures::future::Future<Item = (), Error = ()> {
    let (ws_tx, mut ws_rx) = socket.split();
    let timeline = client_agent.subscription.timeline;

    // Create a pipe
    let (tx, rx) = futures::sync::mpsc::unbounded();

    // Send one end of it to a different thread and tell that end to forward whatever it gets
    // on to the websocket client
    warp::spawn(
        rx.map_err(|()| -> warp::Error { unreachable!() })
            .forward(ws_tx)
            .map(|_r| ())
            .map_err(|e| match e.to_string().as_ref() {
                "IO error: Broken pipe (os error 32)" => (), // just closed unix socket
                _ => log::warn!("websocket send error: {}", e),
            }),
    );

    // Yield new events for as long as the client is still connected
    let event_stream = tokio::timer::Interval::new(time::Instant::now(), update_interval)
        .take_while(move |_| match ws_rx.poll() {
            Ok(Async::NotReady) | Ok(Async::Ready(Some(_))) => futures::future::ok(true),
            Ok(Async::Ready(None)) => {
                // TODO: consider whether we should manually drop closed connections here
                log::info!("Client closed WebSocket connection for {:?}", timeline);
                futures::future::ok(false)
            }
            Err(e) if e.to_string() == "IO error: Broken pipe (os error 32)" => {
                // no err, just closed Unix socket
                log::info!("Client closed WebSocket connection for {:?}", timeline);
                futures::future::ok(false)
            }
            Err(e) => {
                log::warn!("Error in {:?}: {}", timeline, e);
                futures::future::ok(false)
            }
        });

    let mut time = time::Instant::now();

    // Every time you get an event from that stream, send it through the pipe
    event_stream
        .for_each(move |_instant| {
            if let Ok(Async::Ready(Some(msg))) = client_agent.poll() {
                tx.unbounded_send(warp::ws::Message::text(
                    json!({ "event": msg.event(),
                          "payload": msg.payload() })
                    .to_string(),
                ))
                .expect("No send error");
            };
            if time.elapsed() > time::Duration::from_secs(30) {
                tx.unbounded_send(warp::ws::Message::text("{}"))
                    .expect("Can ping");
                time = time::Instant::now();
            }
            Ok(())
        })
        .then(move |result| {
            // TODO: consider whether we should manually drop closed connections here
            log::info!("WebSocket connection for {:?} closed.", timeline);
            result
        })
        .map_err(move |e| log::warn!("Error sending to {:?}: {}", timeline, e))
}
