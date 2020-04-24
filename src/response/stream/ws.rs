use super::{Event, Payload};
use crate::request::Subscription;

use futures::future::Future;
use futures::stream::Stream;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use warp::ws::{Message, WebSocket};

type EventRx = Receiver<Arc<Event>>;

pub struct Ws(Subscription);

impl Ws {
    pub fn new(subscription: Subscription) -> Self {
        Self(subscription)
    }

    pub fn send_to(
        mut self,
        ws: WebSocket,
        event_rx: EventRx,
    ) -> impl Future<Item = (), Error = ()> {
        let (transmit_to_ws, _receive_from_ws) = ws.split();
        event_rx
            .filter_map(move |event| {
                if matches!(*event, Event::Ping) {
                    Some(Message::text(&event.to_json_string()))
                } else {
                    match (event.update_payload(), event.dyn_update_payload()) {
                        (Some(update), _) if !self.filtered(update) => {
                            Some(Message::text(&event.to_json_string()))
                        }
                        (None, None) => Some(Message::text(&event.to_json_string())), // send all non-updates
                        (_, Some(dyn_update)) if !self.filtered(dyn_update) => {
                            Some(Message::text(&event.to_json_string()))
                        }
                        _ => None,
                    }
                }
            })
            .map_err(|_| -> warp::Error { unreachable!() })
            .forward(transmit_to_ws)
            .map(|_r| ())
            // ignore errors that indicate normal disconnects.  TODO - once we upgrade our
            // Warp version, we should stop matching on text, which is fragile.
            .map_err(|e| match e.to_string().as_ref() {
                "IO error: Broken pipe (os error 32)"
                | "IO error: Connection reset by peer (os error 104)" => (),
                e => log::warn!("WebSocket send error: {}", e),
            })
    }
    fn filtered(&mut self, update: &impl Payload) -> bool {
        let (blocks, allowed_langs) = (&self.0.blocks, &self.0.allowed_langs);
        let skip = |msg| Some(log::info!("{:?} msg skipped - {}", self.0.timeline, msg)).is_some();

        match self.0.timeline {
            tl if tl.is_public()
                && !update.language_unset()
                && !allowed_langs.is_empty()
                && !allowed_langs.contains(&update.language()) =>
            {
                skip("disallowed language")
            }
            _ if !blocks.blocked_users.is_disjoint(&update.involved_users()) => {
                skip("involves blocked user")
            }
            _ if blocks.blocking_users.contains(update.author()) => skip("from blocking user"),
            _ if blocks.blocked_domains.contains(update.sent_from()) => skip("from blocked domain"),
            _ => false,
        }
    }
}
