use crate::messages::Event;
use crate::parse_client_request::{Subscription, Timeline};

use futures::{future::Future, stream::Stream};
use log;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
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
        event_rx: watch::Receiver<(Timeline, Event)>,
        mut subscribe_tx: mpsc::UnboundedSender<Timeline>,
    ) -> impl Future<Item = (), Error = ()> {
        let (transmit_to_ws, _receive_from_ws) = ws.split();
        let target_timeline = subscription.timeline;
        let user_langs = subscription.allowed_langs;
        let blocks = subscription.blocks;

        // Create a pipe
        let (ws_tx, ws_rx) = futures::sync::mpsc::unbounded();

        // Send one end of it to a different green thread and tell that end to forward
        // whatever it gets on to the WebSocket client
        warp::spawn(
            ws_rx
                .map_err(|()| -> warp::Error { unreachable!() })
                .forward(transmit_to_ws)
                .map(|_r| ())
                .map_err(|e| match e.to_string().as_ref() {
                    "IO error: Broken pipe (os error 32)" => (), // just closed unix socket
                    _ => log::warn!("WebSocket send error: {}", e),
                }),
        );

        event_rx.map_err(|_| ()).for_each(move |(tl, event)| {
            if target_timeline == tl {
                use crate::messages::{CheckedEvent::Update, Event::*};
                use crate::parse_client_request::Stream::Public;

                log::info!("Got event for {:?}", tl);
                if let Event::TypeSafe(Update { payload, .. }) = event.clone() {
                    log::info!("{:?}", &payload.content);
                }
                match event {
                    Ping => match ws_tx.unbounded_send(Message::text("{}")) {
                        Ok(_) => Ok(()),
                        Err(_) => {
                            subscribe_tx.try_send(tl).expect("TODO");
                            Err(())
                        }
                    },
                    TypeSafe(Update { payload, queued_at }) => match tl {
                        Timeline(Public, _, _) if payload.language_not(&user_langs) => Ok(()),
                        _ if payload.involves_any(&blocks) => Ok(()),
                        _ => match ws_tx.unbounded_send(Message::text(
                            TypeSafe(Update { payload, queued_at }).to_json_string(),
                        )) {
                            Ok(_) => Ok(()),
                            Err(_) => {
                                subscribe_tx.try_send(tl).expect("TODO");
                                Err(())
                            }
                        },
                    },
                    TypeSafe(non_update) => match ws_tx
                        .unbounded_send(Message::text(TypeSafe(non_update).to_json_string()))
                    {
                        Ok(_) => Ok(()),
                        Err(_) => {
                            subscribe_tx.try_send(tl).expect("TODO");
                            Err(())
                        }
                    },
                    Dynamic(event) if event.event == "update" => match tl {
                        Timeline(Public, _, _) if event.language_not(&user_langs) => Ok(()),
                        _ if event.involves_any(&blocks) => Ok(()),
                        _ => match ws_tx
                            .unbounded_send(Message::text(Dynamic(event).to_json_string()))
                        {
                            Ok(_) => Ok(()),
                            Err(_) => {
                                subscribe_tx.try_send(tl).expect("TODO");
                                Err(())
                            }
                        },
                    },
                    Dynamic(non_update) => match ws_tx
                        .unbounded_send(Message::text(Dynamic(non_update).to_json_string()))
                    {
                        Ok(_) => Ok(()),
                        Err(_) => {
                            subscribe_tx.try_send(tl).expect("TODO");
                            Err(())
                        }
                    },
                }
            } else {
                if let Event::Ping = event {
                    match ws_tx.unbounded_send(Message::text("{}")) {
                        Ok(_) => Ok(()),
                        Err(_) => {
                            subscribe_tx.try_send(target_timeline).expect("TODO");
                            Err(())
                        }
                    }
                } else {
                    Ok(())
                }
            }
        })

        //                     event_rx
        // .take_while(move |(tl, event)| {
        //                 let (tl, event) = (*tl, event.clone());
        //                 if target_timeline == tl {
        //                     log::info!("Got event for {:?}", tl);
        //                     use crate::messages::{CheckedEvent::Update, Event::*};
        //                     use crate::parse_client_request::Stream::Public;
        //                     match event {
        //                         Ping => match ws_tx.unbounded_send(Message::text("{}")) {
        //                             Ok(_) => futures::future::ok(true),
        //                             Err(_) => {
        //                                 subscribe_tx.try_send(tl).expect("TODO");
        //                                 futures::future::ok(false)
        //                             }
        //                         },
        //                         TypeSafe(Update { payload, queued_at }) => match tl {
        //                             Timeline(Public, _, _) if payload.language_not(&user_langs) => {
        //                                 futures::future::ok(true)
        //                             }
        //                             _ if payload.involves_any(&blocks) => futures::future::ok(true),
        //                             _ => match ws_tx.unbounded_send(Message::text(
        //                                 TypeSafe(Update { payload, queued_at }).to_json_string(),
        //                             )) {
        //                                 Ok(_) => futures::future::ok(true),
        //                                 Err(_) => {
        //                                     subscribe_tx.try_send(tl).expect("TODO");
        //                                     futures::future::ok(false)
        //                                 }
        //                             },
        //                         },
        //                         TypeSafe(non_update) => match ws_tx
        //                             .unbounded_send(Message::text(TypeSafe(non_update).to_json_string()))
        //                         {
        //                             Ok(_) => futures::future::ok(true),
        //                             Err(_) => {
        //                                 subscribe_tx.try_send(tl).expect("TODO");
        //                                 futures::future::ok(false)
        //                             }
        //                         },
        //                         Dynamic(event) if event.event == "update" => match tl {
        //                             Timeline(Public, _, _) if event.language_not(&user_langs) => {
        //                                 futures::future::ok(true)
        //                             }
        //                             _ if event.involves_any(&blocks) => futures::future::ok(true),
        //                             _ => match ws_tx
        //                                 .unbounded_send(Message::text(Dynamic(event).to_json_string()))
        //                             {
        //                                 Ok(_) => futures::future::ok(true),
        //                                 Err(_) => {
        //                                     subscribe_tx.try_send(tl).expect("TODO");
        //                                     futures::future::ok(false)
        //                                 }
        //                             },
        //                         },
        //                         Dynamic(non_update) => match ws_tx
        //                             .unbounded_send(Message::text(Dynamic(non_update).to_json_string()))
        //                         {
        //                             Ok(_) => futures::future::ok(true),
        //                             Err(_) => {
        //                                 subscribe_tx.try_send(tl).expect("TODO");
        //                                 futures::future::ok(false)
        //                             }
        //                         },
        //                     }
        //                 } else {
        //                     if let Event::Ping = event {
        //                         match ws_tx.unbounded_send(Message::text("{}")) {
        //                             Ok(_) => futures::future::ok(true),
        //                             Err(_) => {
        //                                 subscribe_tx.try_send(target_timeline).expect("TODO");
        //                                 futures::future::ok(false)
        //                             }
        //                         }
        //                     } else {
        //                         futures::future::ok(true)
        //                     }
        //                 }
        //             })
        // .for_each(|_| Ok(()))
        // .map_err(|_| ())
        // .map(|_| ())
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
                        _ => Some((
                            warp::sse::event(event.event),
                            warp::sse::data(event.payload.to_string()),
                        )),
                    },
                    Dynamic(non_update) => Some((
                        warp::sse::event(non_update.event),
                        warp::sse::data(non_update.payload.to_string()),
                    )),
                    // TODO: Fix for Ping
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

// if target_timeline == tl {
//     log::info!("Got event for {:?}", tl);
//     use crate::messages::{CheckedEvent::Update, Event::*};
//     use crate::parse_client_request::Stream::Public;
//     match event {
//         TypeSafe(Update { payload, queued_at }) => match tl {
//             Timeline(Public, _, _) if payload.language_not(&user_langs) => Ok(()),
//             _ if payload.involves_any(&blocks) => Ok(()),
//             _ => Ok(ws_tx
//                 .unbounded_send(Message::text(
//                     TypeSafe(Update { payload, queued_at }).to_json_string(),
//                 ))
//                 .unwrap_or_else(|_| subscribe_tx.try_send(tl).expect("TODO"))),
//         },
//         TypeSafe(non_update) => Ok(ws_tx
//             .unbounded_send(Message::text(TypeSafe(non_update).to_json_string()))
//             .unwrap_or_else(|_| subscribe_tx.try_send(tl).expect("TODO"))),
//         Dynamic(event) if event.event == "update" => match tl {
//             Timeline(Public, _, _) if event.language_not(&user_langs) => Ok(()),
//             _ if event.involves_any(&blocks) => Ok(()),
//             _ => Ok(ws_tx
//                 .unbounded_send(Message::text(Dynamic(event).to_json_string()))
//                 .unwrap_or_else(|_| subscribe_tx.try_send(tl).expect("TODO"))),
//         },
//         Dynamic(non_update) => Ok(ws_tx
//             .unbounded_send(Message::text(Dynamic(non_update).to_json_string()))
//             .unwrap_or_else(|_| subscribe_tx.try_send(tl).expect("TODO"))),
//         Ping => Ok(match ws_tx.unbounded_send(Message::text("{}")) {
//             Ok(_) => (),
//             Err(_) => {
//                 subscribe_tx.try_send(tl).expect("TODO");
//             }
//         }),
//     }
// } else {
//     if let Event::Ping = event {
//         Ok(ws_tx
//             .unbounded_send(Message::text("{}"))
//             .unwrap_or_else(|_| {
//                 subscribe_tx.try_send(target_timeline).expect("TODO")
//             }))
//     } else {
//         Ok(())
//     }
// }
