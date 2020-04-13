use crate::event::Event;
use crate::request::{Subscription, Timeline};

use futures::stream::Stream;
use log;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use warp::reply::Reply;
use warp::sse::{ServerSentEvent, Sse as WarpSse};

pub struct Sse;

impl Sse {
    fn reply_with(event: Event) -> Option<(impl ServerSentEvent, impl ServerSentEvent)> {
        Some((
            warp::sse::event(event.event_name()),
            warp::sse::data(event.payload().unwrap_or_else(String::new)),
        ))
    }

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
                use crate::event::{
                    CheckedEvent, CheckedEvent::Update, DynEvent, Event::*, EventKind,
                };

                use crate::request::Stream::Public;
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
                    Dynamic(dyn_event) => {
                        if let EventKind::Update(s) = dyn_event.kind {
                            match timeline {
                                Timeline(Public, _, _) if s.language_not(&allowed_langs) => None,
                                _ if s.involves_any(&blocks) => None,
                                _ => Self::reply_with(Dynamic(DynEvent {
                                    kind: EventKind::Update(s),
                                    ..dyn_event
                                })),
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
