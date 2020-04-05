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
    pub fn send_to_ws(
        ws: WebSocket,
        mut client_agent: ClientAgent,
        interval: Duration,
    ) -> impl Future<Item = (), Error = ()> {
        let (transmit_to_ws, _receive_from_ws) = ws.split();
        let timeline = client_agent.subscription.timeline;

        // Create a pipe
        let (tx, rx) = futures::sync::mpsc::unbounded();

        // Send one end of it to a different thread and tell that end to forward whatever it gets
        // on to the WebSocket client
        warp::spawn(
            rx.map_err(|()| -> warp::Error { unreachable!() })
                .forward(transmit_to_ws)
                .map(|_r| ())
                .map_err(|e| match e.to_string().as_ref() {
                    "IO error: Broken pipe (os error 32)" => (), // just closed unix socket
                    _ => log::warn!("WebSocket send error: {}", e),
                }),
        );

        let mut last_ping_time = Instant::now();
        tokio::timer::Interval::new(Instant::now(), interval)
            .take_while(move |_| {
                // Right now, we do not need to see if we have any messages _from_ the
                // WebSocket connection because the API doesn't support clients sending
                // commands via the WebSocket.  However, if the [stream multiplexing API
                // change](github.com/tootsuite/flodgatt/issues/121) is implemented, we'll
                // need to receive messages from the client.  If so, we'll need a
                // `receive_from_ws.poll() call here (or later)`

                match client_agent.poll() {
                    Ok(Async::Ready(Some(msg))) => {
                        match tx.unbounded_send(Message::text(msg.to_json_string())) {
                            Ok(_) => futures::future::ok(true),
                            Err(_) => client_agent.disconnect(),
                        }
                    }
                    Ok(Async::Ready(None)) => {
                        log::info!("WebSocket ClientAgent got Ready(None)");
                        futures::future::ok(true)
                    }
                    Ok(Async::NotReady) if last_ping_time.elapsed() > Duration::from_secs(30) => {
                        last_ping_time = Instant::now();
                        match tx.unbounded_send(Message::text("{}")) {
                            Ok(_) => futures::future::ok(true),
                            Err(_) => client_agent.disconnect(),
                        }
                    }
                    Ok(Async::NotReady) => futures::future::ok(true), // no new messages; nothing to do
                    Err(e) => {
                        log::error!("{}\n Dropping WebSocket message and continuing.", e);
                        futures::future::ok(true)
                    }
                }
            })
            .for_each(move |_instant| Ok(()))
            .then(move |result| {
                log::info!("WebSocket connection for {:?} closed.", timeline);
                result
            })
            .map_err(move |e| log::warn!("Error sending to {:?}: {}", timeline, e))
    }

    pub fn send_to_sse(mut client_agent: ClientAgent, sse: Sse, interval: Duration) -> impl Reply {
        let event_stream =
            tokio::timer::Interval::new(Instant::now(), interval).filter_map(move |_| {
                match client_agent.poll() {
                    Ok(Async::Ready(Some(event))) => Some((
                        warp::sse::event(event.event_name()),
                        warp::sse::data(event.payload().unwrap_or_else(String::new)),
                    )),
                    Ok(Async::Ready(None)) => {
                        log::info!("SSE ClientAgent got Ready(None)");
                        None
                    }
                    Ok(Async::NotReady) => None,
                    Err(e) => {
                        log::error!("{}\n Dropping SSE message and continuing.", e);
                        None
                    }
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
