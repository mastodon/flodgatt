//! WebSocket-specific functionality
use crate::query;
use crate::stream_manager::StreamManager;
use crate::user::{Scope, User};
use crate::user_from_path;
use futures::future::Future;
use futures::stream::Stream;
use futures::Async;
use std::time;
use warp::filters::BoxedFilter;
use warp::{path, Filter};

/// Send a stream of replies to a WebSocket client
pub fn send_replies(
    socket: warp::ws::WebSocket,
    mut stream: StreamManager,
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
    let event_stream =
        tokio::timer::Interval::new(time::Instant::now(), time::Duration::from_millis(100))
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

pub fn websocket_routes() -> BoxedFilter<(User, Query, warp::ws::Ws2)> {
    user_from_path!("streaming", Scope::Public)
        .and(warp::query())
        .and(query::Media::to_filter())
        .and(query::Hashtag::to_filter())
        .and(query::List::to_filter())
        .and(warp::ws2())
        .map(
            |user: User,
             stream: query::Stream,
             media: query::Media,
             hashtag: query::Hashtag,
             list: query::List,
             ws: warp::ws::Ws2| {
                let query = Query {
                    stream: stream.stream,
                    media: media.is_truthy(),
                    hashtag: hashtag.tag,
                    list: list.list,
                };
                (user, query, ws)
            },
        )
        .untuple_one()
        .boxed()
}

#[derive(Debug)]
pub struct Query {
    pub stream: String,
    pub media: bool,
    pub hashtag: String,
    pub list: i64,
}
