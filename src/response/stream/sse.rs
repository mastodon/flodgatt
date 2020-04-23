use super::{Event, Payload};
use crate::request::Subscription;

use futures::stream::Stream;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use warp::reply::Reply;
use warp::sse::Sse as WarpSse;

type EventRx = UnboundedReceiver<Event>;

pub struct Sse(Subscription);

impl Sse {
    pub fn new(subscription: Subscription) -> Self {
        Self(subscription)
    }

    pub fn send_events(self, sse: WarpSse, event_rx: EventRx) -> impl Reply {
        let event_stream = event_rx.filter_map(move |event| {
            match (event.update_payload(), event.dyn_update_payload()) {
                (Some(update), _) if self.update_not_filtered(update) => event.to_warp_reply(),
                (_, Some(update)) if self.update_not_filtered(update) => event.to_warp_reply(),
                (_, _) => event.to_warp_reply(), // send all non-updates
            }
        });

        sse.reply(
            warp::sse::keep_alive()
                .interval(Duration::from_secs(30))
                .text("thump".to_string())
                .stream(event_stream),
        )
    }

    fn update_not_filtered(&self, update: &impl Payload) -> bool {
        let blocks = &self.0.blocks;
        let allowed_langs = &self.0.allowed_langs;

        match self.0.timeline {
            tl if tl.is_public()
                && !update.language_unset()
                && !allowed_langs.is_empty()
                && !allowed_langs.contains(&update.language()) =>
            {
                false
            }
            _ if !blocks.blocked_users.is_disjoint(&update.involved_users()) => false,
            _ if blocks.blocking_users.contains(update.author()) => false,
            _ if blocks.blocked_domains.contains(update.sent_from()) => false,
            _ => true,
        }
    }
}
