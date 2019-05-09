//! Streaming server for Mastodon
//!
//!
//! This server provides live, streaming updates for Mastodon clients.  Specifically, when a server
//! is running this sever, Mastodon clients can use either Server Sent Events or WebSockets to
//! connect to the server with the API described [in the public API
//! documentation](https://docs.joinmastodon.org/api/streaming/)
//!
//! # Notes on data flow
//! * **Client Request → Warp**:
//! Warp filters for valid requests and parses request data. Based on that data, it repeatedly polls
//! the StreamManager
//!
//! * **Warp → StreamManager**:
//! The StreamManager consults a hash table to see if there is a currently open PubSub channel. If
//! there is, it uses that channel; if not, it (synchronously) sends a subscribe command to Redis.
//! The StreamManager polls the Receiver, providing info about which StreamManager it is that is
//! doing the polling. The stream manager is also responsible for monitoring the hash table to see
//! if it should unsubscribe from any channels and, if necessary, sending the unsubscribe command.
//!
//! * **StreamManger → Receiver**:
//! The Receiver receives data from Redis and stores it in a series of queues (one for each
//! StreamManager). When (asynchronously) polled by the StreamManager, it sends back the  messages
//! relevant to that StreamManager and removes them from the queue.

pub mod error;
pub mod query;
pub mod receiver;
pub mod stream;
pub mod timeline;
pub mod user;
use futures::stream::Stream;
use receiver::Receiver;
use stream::StreamManager;
use user::{Filter, User};
use warp::Filter as WarpFilter;

fn main() {
    pretty_env_logger::init();

    // let redis_updates = StreamManager::new(Receiver::new());

    // let routes = any_of!(
    //     // GET /api/v1/streaming/user/notification                     [private; notification filter]
    //     timeline::user_notifications(),
    //     // GET /api/v1/streaming/user                                  [private; language filter]
    //     timeline::user(),
    //     // GET /api/v1/streaming/public/local?only_media=true          [public; language filter]
    //     timeline::public_local_media(),
    //     // GET /api/v1/streaming/public?only_media=true                [public; language filter]
    //     timeline::public_media(),
    //     // GET /api/v1/streaming/public/local                          [public; language filter]
    //     timeline::public_local(),
    //     // GET /api/v1/streaming/public                                [public; language filter]
    //     timeline::public(),
    //     // GET /api/v1/streaming/direct                                [private; *no* filter]
    //     timeline::direct(),
    //     // GET /api/v1/streaming/hashtag?tag=:hashtag                  [public; no filter]
    //     timeline::hashtag(),
    //     // GET /api/v1/streaming/hashtag/local?tag=:hashtag            [public; no filter]
    //     timeline::hashtag_local(),
    //     // GET /api/v1/streaming/list?list=:list_id                    [private; no filter]
    //     timeline::list()
    // )
    // .untuple_one()
    // .and(warp::sse())
    // .and(warp::any().map(move || redis_updates.new_copy()))
    // .map(
    //     |timeline: String, user: User, sse: warp::sse::Sse, mut event_stream: StreamManager| {
    //         dbg!(&event_stream);
    //         event_stream.add(&timeline, &user);
    //         sse.reply(warp::sse::keep(
    //             event_stream.filter_map(move |item| {
    //                 let payload = item["payload"].clone();
    //                 let event = item["event"].clone().to_string();
    //                 let toot_lang = payload["language"].as_str().expect("redis str").to_string();
    //                 let user_langs = user.langs.clone();

    //                 match (&user.filter, user_langs) {
    //                     (Filter::Notification, _) if event != "notification" => None,
    //                     (Filter::Language, Some(ref langs)) if !langs.contains(&toot_lang) => None,
    //                     _ => Some((warp::sse::event(event), warp::sse::data(payload))),
    //                 }
    //             }),
    //             None,
    //         ))
    //     },
    // )
    // .with(warp::reply::with::header("Connection", "keep-alive"))
    // .recover(error::handle_errors);

    use futures::future::Future;
    use futures::sink::Sink;
    use futures::Async;
    use user::Scope;
    use warp::path;
    let redis_updates_ws = StreamManager::new(Receiver::new());
    let websocket = path!("api" / "v1" / "streaming")
        .and(Scope::Public.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Public))
        .and(warp::query())
        .and(query::Media::to_filter())
        .and(query::Hashtag::to_filter())
        .and(query::List::to_filter())
        .and(warp::ws2())
        .and(warp::any().map(move || {
            println!("Getting StreamManager.new_copy()");
            redis_updates_ws.new_copy()
        }))
        .and_then(
            |mut user: User,
             q: query::Stream,
             m: query::Media,
             h: query::Hashtag,
             l: query::List,
             ws: warp::ws::Ws2,
             mut stream: StreamManager| {
                println!("DING");
                let unauthorized = Err(warp::reject::custom("Error: Invalid Access Token"));
                let timeline = match q.stream.as_ref() {
                    // Public endpoints:
                    tl @ "public" | tl @ "public:local" if m.is_truthy() => format!("{}:media", tl),
                    tl @ "public:media" | tl @ "public:local:media" => format!("{}", tl),
                    tl @ "public" | tl @ "public:local" => format!("{}", tl),
                    // User
                    "user" if user.id == -1 => return unauthorized,
                    "user" => format!("{}", user.id),
                    "user:notification" => {
                        user = user.with_notification_filter();
                        format!("{}", user.id)
                    }
                    // Hashtag endpoints:
                    // TODO: handle missing query
                    tl @ "hashtag" | tl @ "hashtag:local" => format!("{}:{}", tl, h.tag),
                    // List endpoint:
                    // TODO: handle missing query
                    "list" if user.authorized_for_list(l.list).is_err() => return unauthorized,
                    "list" => format!("list:{}", l.list),
                    // Direct endpoint:
                    "direct" if user.id == -1 => return unauthorized,
                    "direct" => format!("direct"),
                    // Other endpoints don't exist:
                    _ => return Err(warp::reject::custom("Error: Nonexistent WebSocket query")),
                };

                stream.add(&timeline, &user);
                stream.set_user(user);
                dbg!(&stream);
                Ok(ws.on_upgrade(move |socket| handle_ws(socket, stream)))
            },
        );

    fn handle_ws(
        socket: warp::ws::WebSocket,
        mut stream: StreamManager,
    ) -> impl futures::future::Future<Item = (), Error = ()> {
        let (mut tx, rx) = futures::sync::mpsc::unbounded();
        let (ws_tx, mut ws_rx) = socket.split();
        // let event_stream = stream
        //     .map(move |value| warp::ws::Message::text(value.to_string()))
        //     .map_err(|_| unreachable!());
        warp::spawn(
            rx.map_err(|()| -> warp::Error { unreachable!() })
                .forward(ws_tx)
                .map_err(|_| ())
                .map(|_r| ()),
        );
        let event_stream = tokio::timer::Interval::new(
            std::time::Instant::now(),
            std::time::Duration::from_secs(10),
        )
        .take_while(move |_| {
            if ws_rx.poll().is_err() {
                println!("Need to close WS");
                futures::future::ok(false)
            } else {
                // println!("We can still send to WS");
                futures::future::ok(true)
            }
        });

        event_stream
            .for_each(move |_json_value| {
                // println!("For each triggered");
                if let Ok(Async::Ready(Some(json_value))) = stream.poll() {
                    let msg = warp::ws::Message::text(json_value.to_string());
                    tx.unbounded_send(msg).unwrap();
                };
                Ok(())
            })
            .then(|msg| {
                println!("Done with stream");
                msg
            })
            .map_err(|e| {
                println!("{}", e);
            })
    }

    let log = warp::any().map(|| {
        println!("----got request----");
        warp::reply()
    });
    warp::serve(websocket.or(log)).run(([127, 0, 0, 1], 3030));
}

// loop {
//     //println!("Awake");
//     match stream.poll() {
//         Err(_) | Ok(Async::Ready(None)) => {
//             eprintln!("Breaking out of poll loop due to an error");
//             break;
//         }
//         Ok(Async::NotReady) => (),
//         Ok(Async::Ready(Some(item))) => {
//             let user_langs = user.langs.clone();
//             let copy = item.clone();
//             let event = copy["event"].as_str().unwrap();
//             let copy = item.clone();
//             let payload = copy["payload"].to_string();
//             let copy = item.clone();
//             let toot_lang = copy["payload"]["language"]
//                 .as_str()
//                 .expect("redis str")
//                 .to_string();

//             println!("sending: {:?}", &payload);
//             match (&user.filter, user_langs) {
//                 (Filter::Notification, _) if event != "notification" => continue,
//                 (Filter::Language, Some(ref langs)) if !langs.contains(&toot_lang) => {
//                     continue;
//                 }
//                 _ => match tx.unbounded_send(warp::ws::Message::text(
//                     json!(
//                         {"event": event,
//                          "payload": payload,}
//                     )
//                     .to_string(),
//                 )) {
//                     Ok(()) => println!("Sent OK"),
//                     Err(e) => {
//                         println!("Couldn't send: {}", e);
//                     }
//                 },
//             }
//         }
//     };
//     if ws_rx.poll().is_err() {
//         println!("Need to close WS");
//         break;
//     } else {
//         println!("We can still send to WS");
//     }
//     std::thread::sleep(std::time::Duration::from_millis(2000));
//     //println!("Asleep");
// }
