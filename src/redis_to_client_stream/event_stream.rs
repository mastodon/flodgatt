use crate::messages::Event;
use crate::parse_client_request::{Subscription, Timeline};

use futures::{future::Future, stream::Stream};
use log;
use std::time::Duration;
use tokio::sync::watch;
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
        subscription: Subscription,
        ws_rx: watch::Receiver<(Timeline, Event)>,
    ) -> impl Future<Item = (), Error = ()> {
        let (transmit_to_ws, _receive_from_ws) = ws.split();
        let target_timeline = subscription.timeline;
        let allowed_langs = subscription.allowed_langs;
        let blocks = subscription.blocks;

        // Create a pipe
        let (tx, rx) = futures::sync::mpsc::unbounded();

        // Send one end of it to a different green thread and tell that end to forward
        // whatever it gets on to the WebSocket client
        warp::spawn(
            rx.map_err(|()| -> warp::Error { unreachable!() })
                .forward(transmit_to_ws)
                .map(|_r| ())
                .map_err(|e| match e.to_string().as_ref() {
                    "IO error: Broken pipe (os error 32)" => (), // just closed unix socket
                    _ => log::warn!("WebSocket send error: {}", e),
                }),
        );

        return ws_rx
            .for_each(move |(timeline, event)| {
                if target_timeline == timeline {
                    log::info!("Got event for {:?}", timeline);
                    use crate::messages::{CheckedEvent::Update, Event::*};
                    use crate::parse_client_request::Stream::Public;
                    match event {
                        TypeSafe(Update { payload, queued_at }) => match timeline {
                            Timeline(Public, _, _) if payload.language_not(&allowed_langs) => (),
                            _ if payload.involves_any(&blocks) => (),
                            // TODO filter vvvv
                            _ => tx
                                .unbounded_send(Message::text(
                                    TypeSafe(Update { payload, queued_at }).to_json_string(),
                                ))
                                .expect("TODO"),
                        },
                        TypeSafe(non_update) => tx
                            .unbounded_send(Message::text(TypeSafe(non_update).to_json_string()))
                            .expect("TODO"),
                        Dynamic(event) if event.event == "update" => match timeline {
                            Timeline(Public, _, _) if event.language_not(&allowed_langs) => (),
                            _ if event.involves_any(&blocks) => (),
                            // TODO filter vvvv
                            _ => tx
                                .unbounded_send(Message::text(Dynamic(event).to_json_string()))
                                .expect("TODO"),
                        },
                        Dynamic(non_update) => tx
                            .unbounded_send(Message::text(Dynamic(non_update).to_json_string()))
                            .expect("TODO"),
                        EventNotReady => panic!("TODO"),
                    }
                }
                Ok(())
            })
            .map_err(|_| ());
    }

    pub fn send_to_sse(
        sse: Sse,
        subscription: Subscription,
        sse_rx: watch::Receiver<(Timeline, Event)>,
    ) -> impl Reply {
        let target_timeline = subscription.timeline;
        let allowed_langs = subscription.allowed_langs;
        let blocks = subscription.blocks;

        let event_stream = sse_rx.filter_map(move |(timeline, event)| {
            if target_timeline == timeline {
                log::info!("Got event for {:?}", timeline);
                use crate::messages::{CheckedEvent, CheckedEvent::Update, Event::*};
                use crate::parse_client_request::Stream::Public;
                match event {
                    TypeSafe(Update { payload, queued_at }) => match timeline {
                        Timeline(Public, _, _) if payload.language_not(&allowed_langs) => None,
                        _ if payload.involves_any(&blocks) => None,
                        // TODO filter vvvv
                        _ => {
                            let event =
                                Event::TypeSafe(CheckedEvent::Update { payload, queued_at });
                            Some((
                                warp::sse::event(event.event_name()),
                                warp::sse::data(event.payload().unwrap_or_else(String::new)),
                            ))
                        }
                    },
                    TypeSafe(non_update) => {
                        let event = Event::TypeSafe(non_update);
                        Some((
                            warp::sse::event(event.event_name()),
                            warp::sse::data(event.payload().unwrap_or_else(String::new)),
                        ))
                    }
                    Dynamic(event) if event.event == "update" => match timeline {
                        Timeline(Public, _, _) if event.language_not(&allowed_langs) => None,
                        _ if event.involves_any(&blocks) => None,
                        // TODO filter vvvv
                        _ => {
                            let event = Event::Dynamic(event);
                            Some((
                                warp::sse::event(event.event_name()),
                                warp::sse::data(event.payload().unwrap_or_else(String::new)),
                            ))
                        }
                    },
                    Dynamic(non_update) => {
                        let event = Event::Dynamic(non_update);
                        Some((
                            warp::sse::event(event.event_name()),
                            warp::sse::data(event.payload().unwrap_or_else(String::new)),
                        ))
                    }
                    EventNotReady => panic!("TODO"),
                }
            } else {
                None
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
