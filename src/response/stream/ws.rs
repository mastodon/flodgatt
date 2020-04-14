use crate::event::Event;
use crate::request::{Subscription, Timeline};

use futures::{future::Future, stream::Stream};
use tokio::sync::{mpsc, watch};
use warp::ws::{Message, WebSocket};

pub struct Ws {
    unsubscribe_tx: mpsc::UnboundedSender<Timeline>,
    subscription: Subscription,
    ws_rx: watch::Receiver<(Timeline, Event)>,
    ws_tx: Option<mpsc::UnboundedSender<Message>>,
}

impl Ws {
    pub fn new(
        unsubscribe_tx: mpsc::UnboundedSender<Timeline>,
        ws_rx: watch::Receiver<(Timeline, Event)>,
        subscription: Subscription,
    ) -> Self {
        Self {
            unsubscribe_tx,
            subscription,
            ws_rx,
            ws_tx: None,
        }
    }

    pub fn send_to(mut self, ws: WebSocket) -> impl Future<Item = (), Error = ()> {
        let (transmit_to_ws, _receive_from_ws) = ws.split();
        // Create a pipe
        let (ws_tx, ws_rx) = mpsc::unbounded_channel();
        self.ws_tx = Some(ws_tx);

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

        let target_timeline = self.subscription.timeline;
        let incoming_events = self.ws_rx.clone().map_err(|_| ());

        incoming_events.for_each(move |(tl, event)| {
            if matches!(event, Event::Ping) {
                self.send_msg(event)
            } else if target_timeline == tl {
                use crate::event::{CheckedEvent::Update, Event::*, EventKind};
                use crate::request::Stream::Public;
                let blocks = &self.subscription.blocks;
                let allowed_langs = &self.subscription.allowed_langs;

                match event {
                    TypeSafe(Update { payload, queued_at }) => match tl {
                        Timeline(Public, _, _) if payload.language_not(allowed_langs) => Ok(()),
                        _ if payload.involves_any(&blocks) => Ok(()),
                        _ => self.send_msg(TypeSafe(Update { payload, queued_at })),
                    },
                    TypeSafe(non_update) => self.send_msg(TypeSafe(non_update)),
                    Dynamic(dyn_event) => {
                        if let EventKind::Update(s) = dyn_event.kind.clone() {
                            match tl {
                                Timeline(Public, _, _) if s.language_not(allowed_langs) => Ok(()),
                                _ if s.involves_any(&blocks) => Ok(()),
                                _ => self.send_msg(Dynamic(dyn_event)),
                            }
                        } else {
                            self.send_msg(Dynamic(dyn_event))
                        }
                    }
                    Ping => unreachable!(), // handled pings above
                }
            } else {
                Ok(())
            }
        })
    }

    fn send_msg(&mut self, event: Event) -> Result<(), ()> {
        let txt = &event.to_json_string();
        let tl = self.subscription.timeline;
        match self.ws_tx.clone().ok_or(())?.try_send(Message::text(txt)) {
            Ok(_) => Ok(()),
            Err(_) => {
                self.unsubscribe_tx
                    .try_send(tl)
                    .unwrap_or_else(|e| log::error!("could not unsubscribe from channel: {}", e));
                Err(())
            }
        }
    }
}
