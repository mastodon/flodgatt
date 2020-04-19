use crate::event::{Event, Payload};
use crate::request::{Subscription, Timeline};

use futures::{future::Future, stream::Stream};
use tokio::sync::{mpsc, watch};
use warp::ws::{Message, WebSocket};

type Result<T> = std::result::Result<T, ()>;

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
                .map_err(|e| {
                    match e.to_string().as_ref() {
                        "IO error: Broken pipe (os error 32)" => (), // just closed unix socket
                        _ => log::warn!("WebSocket send error: {}", e),
                    }
                }),
        );

        let target_timeline = self.subscription.timeline;
        let incoming_events = self.ws_rx.clone().map_err(|_| ());

        incoming_events.for_each(move |(tl, event)| {
            if matches!(event, Event::Ping) {
                self.send_msg(&event)?
            } else if target_timeline == tl {
                match (event.update_payload(), event.dyn_update_payload()) {
                    (Some(update), _) => self.send_or_filter(tl, &event, update)?,
                    (None, None) => self.send_msg(&event)?, // send all non-updates
                    (_, Some(dyn_update)) => self.send_or_filter(tl, &event, dyn_update)?,
                }
            }
            Ok(())
        })
    }

    fn send_or_filter(&mut self, tl: Timeline, event: &Event, update: &impl Payload) -> Result<()> {
        let blocks = &self.subscription.blocks;
        let allowed_langs = &self.subscription.allowed_langs;
        const SKIP: Result<()> = Ok(());
        match tl {
            tl if tl.is_public()
                && !update.language_unset()
                && !allowed_langs.is_empty()
                && !allowed_langs.contains(&update.language()) =>
            {
                SKIP
            }
            _ if !blocks.blocked_users.is_disjoint(&update.involved_users()) => SKIP,
            _ if blocks.blocking_users.contains(update.author()) => SKIP,
            _ if blocks.blocked_domains.contains(update.sent_from()) => SKIP,
            _ => Ok(self.send_msg(&event)?),
        }
    }

    fn send_msg(&mut self, event: &Event) -> Result<()> {
        let txt = &event.to_json_string();
        let tl = self.subscription.timeline;
        let mut channel = self.ws_tx.clone().ok_or(())?;
        channel.try_send(Message::text(txt)).map_err(|_| {
            self.unsubscribe_tx
                .try_send(tl)
                .unwrap_or_else(|e| log::error!("could not unsubscribe from channel: {}", e));
        })
    }
}
