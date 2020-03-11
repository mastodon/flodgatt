//! Stream the updates appropriate for a given `User`/`timeline` pair from Redis.
pub mod client_agent;
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
            Ok(Async::Ready(Some(toot))) => Some((
                warp::sse::event(toot.category),
                warp::sse::data(toot.payload),
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

    let (tl, email, id) = (
        client_agent.current_user.target_timeline.clone(),
        client_agent.current_user.email.clone(),
        client_agent.current_user.id,
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
            Err(e) if e.to_string() == "IO error: Broken pipe (os error 32)" => {
                // no err, just closed Unix socket
                log::info!("Client closed WebSocket connection");
                futures::future::ok(false)
            }
            Err(e) => {
                log::warn!("Error in TL {}\nfor user: {}({})\n{}", tl, email, id, e);
                futures::future::ok(false)
            }
        });

    let mut time = time::Instant::now();

    let (tl, email, id) = (
        client_agent.current_user.target_timeline.clone(),
        client_agent.current_user.email.clone(),
        client_agent.current_user.id,
    );
    // Every time you get an event from that stream, send it through the pipe
    event_stream
        .for_each(move |_instant| {
            if let Ok(Async::Ready(Some(toot))) = client_agent.poll() {
                let txt = &toot.payload["content"];
                log::warn!("toot: {}\n in TL: {}\nuser: {}({})", txt, tl, email, id);

                let msg = warp::ws::Message::text(
                    json!({"event": toot.category,
                           "payload": toot.payload.to_string()})
                    .to_string(),
                );

                tx.unbounded_send(msg).expect("No send error");
            };
            if time.elapsed() > time::Duration::from_secs(30) {
                let msg = warp::ws::Message::text("{}");
                tx.unbounded_send(msg).expect("Can ping");
                time = time::Instant::now();
            }
            Ok(())
        })
        .then(move |result| {
            // TODO: consider whether we should manually drop closed connections here
            log::info!("WebSocket connection closed.");
            result
        })
        .map_err(move |e| log::warn!("Error sending to user: {}\n{}", id, e))
}
