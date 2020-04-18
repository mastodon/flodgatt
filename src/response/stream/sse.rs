use crate::event::Event;
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
        let allowed_langs = subscription.allowed_langs;
        let blocks = subscription.blocks;

        let event_stream = sse_rx
            .filter(move |(timeline, _)| target_timeline == *timeline)
            .filter_map(move |(timeline, event)| {
                use crate::event::Payload;
                use crate::event::{
                    CheckedEvent, CheckedEvent::Update, DynEvent, Event::*, EventKind,
                }; // TODO -- move up

                match event {
                    TypeSafe(Update { payload, queued_at }) => match timeline {
                        tl if tl.is_public()
                            && !payload.language_unset()
                            && !allowed_langs.is_empty()
                            && !allowed_langs.contains(&payload.language()) =>
                        {
                            None
                        }
                        _ if blocks.blocked_users.is_disjoint(&payload.involved_users()) => None,
                        _ if blocks.blocking_users.contains(payload.author()) => None,
                        _ if blocks.blocked_domains.contains(payload.sent_from()) => None,

                        _ => Event::TypeSafe(CheckedEvent::Update { payload, queued_at })
                            .to_warp_reply(),
                    },
                    TypeSafe(non_update) => Event::TypeSafe(non_update).to_warp_reply(),
                    Dynamic(dyn_event) => {
                        if let EventKind::Update(s) = dyn_event.kind {
                            match timeline {
                                tl if tl.is_public()
                                    && !s.language_unset()
                                    && !allowed_langs.is_empty()
                                    && !allowed_langs.contains(&s.language()) =>
                                {
                                    None
                                }
                                _ if blocks.blocked_users.is_disjoint(&s.involved_users()) => None,
                                _ if blocks.blocking_users.contains(s.author()) => None,
                                _ if blocks.blocked_domains.contains(s.sent_from()) => None,

                                _ => Dynamic(DynEvent {
                                    kind: EventKind::Update(s),
                                    ..dyn_event
                                })
                                .to_warp_reply(),
                            }
                        } else {
                            None
                        }
                    }
                    Ping => None, // pings handled automatically
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
}
