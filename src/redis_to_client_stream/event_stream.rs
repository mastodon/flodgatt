use super::ClientAgent;

use futures::{future::Future, stream::Stream, Async};
use log;
use std::time::{Duration, Instant};
use warp::{
    reply::Reply,
    sse::Sse,
    ws::{Message, WebSocket},
};

pub struct EventStream;

impl EventStream {
    /// Send a stream of replies to a WebSocket client.
    pub fn to_ws(
        ws: WebSocket,
        mut client_agent: ClientAgent,
        interval: Duration,
    ) -> impl Future<Item = (), Error = ()> {
        let (ws_tx, mut ws_rx) = ws.split();
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
        let event_stream =
            tokio::timer::Interval::new(Instant::now(), interval).take_while(move |_| {
                match ws_rx.poll() {
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
                }
            });

        let mut time = Instant::now();
        // Every time you get an event from that stream, send it through the pipe
        event_stream
            .for_each(move |_instant| {
                if let Ok(Async::Ready(Some(msg))) = client_agent.poll() {
                    tx.unbounded_send(Message::text(msg.to_json_string()))
                        .expect("No send error");
                };
                if time.elapsed() > Duration::from_secs(30) {
                    tx.unbounded_send(Message::text("{}")).expect("Can ping");
                    time = Instant::now();
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
    pub fn to_sse(mut client_agent: ClientAgent, sse: Sse, interval: Duration) -> impl Reply {
        let event_stream =
            tokio::timer::Interval::new(Instant::now(), interval).filter_map(move |_| {
                match client_agent.poll() {
                    Ok(Async::Ready(Some(event))) => Some((
                        warp::sse::event(event.event_name()),
                        warp::sse::data(event.payload().unwrap_or_else(String::new)),
                    )),
                    _ => None,
                }
            });

        sse.reply(
            warp::sse::keep_alive()
                .interval(Duration::from_secs(30))
                .text("thump".to_string())
                .stream(event_stream),
        )
    }
}
