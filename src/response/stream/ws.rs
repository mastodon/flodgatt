use super::{Event, Payload};
use crate::request::Subscription;

use futures::future::Future;
use futures::stream::Stream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use warp::ws::{Message, WebSocket};

type EventRx = UnboundedReceiver<Event>;
type MsgTx = UnboundedSender<Message>;

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
        // Create a pipe, send one end of it to a different green thread and tell that end
        // to forward to the WebSocket client
        let (mut ws_tx, ws_rx) = mpsc::unbounded_channel();
        warp::spawn(
            ws_rx
                .map_err(|_| -> warp::Error { unreachable!() })
                .forward(transmit_to_ws)
                .map(|_r| ())
                .map_err(|e| {
                    match e.to_string().as_ref() {
                        "IO error: Broken pipe (os error 32)" => (), // just closed unix socket
                        _ => log::warn!("WebSocket send error: {}", e),
                    }
                }),
        );

        event_rx.map_err(|_| ()).for_each(move |event| {
            if matches!(event, Event::Ping) {
                send_msg(&event, &mut ws_tx)?
            } else {
                match (event.update_payload(), event.dyn_update_payload()) {
                    (Some(update), _) => self.send_or_filter(&event, update, &mut ws_tx),
                    (None, None) => send_msg(&event, &mut ws_tx), // send all non-updates
                    (_, Some(dyn_update)) => self.send_or_filter(&event, dyn_update, &mut ws_tx),
                }?
            }
            Ok(())
        })
    }

    fn send_or_filter(
        &mut self,
        event: &Event,
        update: &impl Payload,
        mut ws_tx: &mut MsgTx,
    ) -> Result<(), ()> {
        let (blocks, allowed_langs) = (&self.0.blocks, &self.0.allowed_langs);

        let skip = |reason, tl| Ok(log::info!("{:?} msg skipped - {}", tl, reason));

        match self.0.timeline {
            tl if tl.is_public()
                && !update.language_unset()
                && !allowed_langs.is_empty()
                && !allowed_langs.contains(&update.language()) =>
            {
                skip("disallowed language", tl)
            }

            tl if !blocks.blocked_users.is_disjoint(&update.involved_users()) => {
                skip("involves blocked user", tl)
            }
            tl if blocks.blocking_users.contains(update.author()) => skip("from blocking user", tl),
            tl if blocks.blocked_domains.contains(update.sent_from()) => {
                skip("from blocked domain", tl)
            }
            _ => Ok(send_msg(event, &mut ws_tx)?),
        }
    }
}

fn send_msg(event: &Event, ws_tx: &mut MsgTx) -> Result<(), ()> {
    ws_tx
        .try_send(Message::text(&event.to_json_string()))
        .map_err(|_| log::info!("WebSocket connection closed"))
}
