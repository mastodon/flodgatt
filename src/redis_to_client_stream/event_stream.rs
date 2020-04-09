use crate::messages::Event;
use crate::parse_client_request::{Subscription, Timeline};

use futures::{future::Future, stream::Stream};
use log;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use warp::{
    reply::Reply,
    sse::{ServerSentEvent, Sse},
    ws::{Message, WebSocket},
};

pub struct WsStream {
    ws_tx: mpsc::UnboundedSender<Message>,
    unsubscribe_tx: mpsc::UnboundedSender<Timeline>,
    subscription: Subscription,
}

impl WsStream {
    pub fn new(
        ws: WebSocket,
        unsubscribe_tx: mpsc::UnboundedSender<Timeline>,
        subscription: Subscription,
    ) -> Self {
        let (transmit_to_ws, _receive_from_ws) = ws.split();
        // Create a pipe
        let (ws_tx, ws_rx) = mpsc::unbounded_channel();

        // Send one end of it to a different green thread and tell that end to forward
        // whatever it gets on to the WebSocket client
        warp::spawn(
            ws_rx
                .map_err(|_| -> warp::Error { unreachable!() })
                .forward(transmit_to_ws)
                .map(|_r| ())
                .map_err(|e| match e.to_string().as_ref() {
                    "IO error: Broken pipe (os error 32)" => (), // just closed unix socket
                    _ => log::warn!("WebSocket send error: {}", e),
                }),
        );
        Self {
            ws_tx,
            unsubscribe_tx,
            subscription,
        }
    }

    pub fn send_events(
        mut self,
        event_rx: watch::Receiver<(Timeline, Event)>,
    ) -> impl Future<Item = (), Error = ()> {
        let target_timeline = self.subscription.timeline;

        event_rx.map_err(|_| ()).for_each(move |(tl, event)| {
            if matches!(event, Event::Ping) {
                self.send_ping()
            } else if target_timeline == tl {
                use crate::messages::{CheckedEvent::Update, Event::*};
                use crate::parse_client_request::Stream::Public;
                let blocks = &self.subscription.blocks;
                let allowed_langs = &self.subscription.allowed_langs;

                match event {
                    TypeSafe(Update { payload, queued_at }) => match tl {
                        Timeline(Public, _, _) if payload.language_not(allowed_langs) => Ok(()),
                        _ if payload.involves_any(&blocks) => Ok(()),
                        _ => self.send_msg(TypeSafe(Update { payload, queued_at })),
                    },
                    TypeSafe(non_update) => self.send_msg(TypeSafe(non_update)),
                    Dynamic(event) if event.event == "update" => match tl {
                        Timeline(Public, _, _) if event.language_not(allowed_langs) => Ok(()),
                        _ if event.involves_any(&blocks) => Ok(()),
                        _ => self.send_msg(Dynamic(event)),
                    },
                    Dynamic(non_update) => self.send_msg(Dynamic(non_update)),
                    Ping => unreachable!(), // handled pings above
                }
            } else {
                Ok(())
            }
        })
    }

    fn send_ping(&mut self) -> Result<(), ()> {
        self.send_txt("{}")
    }

    fn send_msg(&mut self, event: Event) -> Result<(), ()> {
        self.send_txt(&event.to_json_string())
    }

    fn send_txt(&mut self, txt: &str) -> Result<(), ()> {
        let tl = self.subscription.timeline;
        match self.ws_tx.try_send(Message::text(txt)) {
            Ok(_) => Ok(()),
            Err(_) => {
                self.unsubscribe_tx.try_send(tl).expect("TODO");
                Err(())
            }
        }
    }
}

pub struct SseStream {}

impl SseStream {
    fn reply_with(event: Event) -> Option<(impl ServerSentEvent, impl ServerSentEvent)> {
        Some((
            warp::sse::event(event.event_name()),
            warp::sse::data(event.payload().unwrap_or_else(String::new)),
        ))
    }

    pub fn send_events(
        sse: Sse,
        mut unsubscribe_tx: mpsc::UnboundedSender<Timeline>,
        subscription: Subscription,
        sse_rx: watch::Receiver<(Timeline, Event)>,
    ) -> impl Reply {
        let target_timeline = subscription.timeline;
        let allowed_langs = subscription.allowed_langs;
        let blocks = subscription.blocks;

        let event_stream = sse_rx
            .filter_map(move |(timeline, event)| {
                if target_timeline == timeline {
                    use crate::messages::{CheckedEvent, CheckedEvent::Update, Event::*};
                    use crate::parse_client_request::Stream::Public;
                    match event {
                        TypeSafe(Update { payload, queued_at }) => match timeline {
                            Timeline(Public, _, _) if payload.language_not(&allowed_langs) => None,
                            _ if payload.involves_any(&blocks) => None,
                            _ => Self::reply_with(Event::TypeSafe(CheckedEvent::Update {
                                payload,
                                queued_at,
                            })),
                        },
                        TypeSafe(non_update) => Self::reply_with(Event::TypeSafe(non_update)),
                        Dynamic(event) if event.event == "update" => match timeline {
                            Timeline(Public, _, _) if event.language_not(&allowed_langs) => None,
                            _ if event.involves_any(&blocks) => None,
                            _ => Self::reply_with(Event::Dynamic(event)),
                        },
                        Dynamic(non_update) => Self::reply_with(Event::Dynamic(non_update)),
                        Ping => None, // pings handled automatically
                    }
                } else {
                    None
                }
            })
            .then(move |res| {
                unsubscribe_tx.try_send(target_timeline).expect("TODO");
                res
            });

        sse.reply(
            warp::sse::keep_alive()
                .interval(Duration::from_secs(30))
                .text("thump".to_string())
                .stream(event_stream),
        )
    }
}
