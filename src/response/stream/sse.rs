use super::{Event, Payload};
use crate::request::{Subscription, Timeline};

use futures::stream::Stream;
use log;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use warp::reply::Reply;
use warp::sse::Sse as WarpSse;

pub struct Sse;

impl Sse {
    pub fn send_events(
        sse: WarpSse,
        mut unsubscribe_tx: mpsc::UnboundedSender<Timeline>,
        subscription: Subscription,
        sse_rx: watch::Receiver<(Timeline, Event)>,
    ) -> impl Reply {
        let target_timeline = subscription.timeline;

        let event_stream = sse_rx
            .filter(move |(timeline, _)| target_timeline == *timeline)
            .filter_map(move |(_timeline, event)| {
                match (event.update_payload(), event.dyn_update_payload()) {
                    (Some(update), _) if Sse::update_not_filtered(subscription.clone(), update) => {
                        event.to_warp_reply()
                    }
                    (None, None) => event.to_warp_reply(), // send all non-updates
                    (_, Some(update)) if Sse::update_not_filtered(subscription.clone(), update) => {
                        event.to_warp_reply()
                    }
                    (_, _) => None,
                }
            })
            .then(move |res| {
                unsubscribe_tx
                    .try_send(target_timeline)
                    .unwrap_or_else(|e| log::error!("could not unsubscribe from channel: {}", e));
                res
            });

        sse.reply(
            warp::sse::keep_alive()
                .interval(Duration::from_secs(30))
                .text("thump".to_string())
                .stream(event_stream),
        )
    }

    fn update_not_filtered(subscription: Subscription, update: &impl Payload) -> bool {
        let blocks = &subscription.blocks;
        let allowed_langs = &subscription.allowed_langs;

        match subscription.timeline {
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
