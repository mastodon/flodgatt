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
//! Warp filters for valid requests and parses request data. Based on that data, it generates a `User`
//! representing the client that made the request.  The `User` is authenticated, if appropriate.  Warp
//! repeatedly polls the StreamManager for information relevant to the User.
//!
//! * **Warp → StreamManager**:
//! A new `StreamManager` is created for each request.  The `StreamManager` exists to manage concurrent
//! access to the (single) `Receiver`, which it can access behind an `Arc<Mutex>`.  The `StreamManager`
//! polles the `Receiver` for any updates relvant to the current client.  If there are updates, the
//! `StreamManager` filters them with the client's filters and passes any matching updates up to Warp.
//! The `StreamManager` is also responsible for sending `subscribe` commands to Redis (via the
//! `Receiver`) when necessary.
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
pub mod ws;
use futures::stream::Stream;
use receiver::Receiver;
use stream::StreamManager;
use user::{Scope, User};
use warp::path;
use warp::Filter as WarpFilter;

fn main() {
    pretty_env_logger::init();

    let redis_updates = StreamManager::new(Receiver::new());

    let routes = any_of!(
        // GET /api/v1/streaming/user/notification                     [private; notification filter]
        timeline::user_notifications(),
        // GET /api/v1/streaming/user                                  [private; language filter]
        timeline::user(),
        // GET /api/v1/streaming/public/local?only_media=true          [public; language filter]
        timeline::public_local_media(),
        // GET /api/v1/streaming/public?only_media=true                [public; language filter]
        timeline::public_media(),
        // GET /api/v1/streaming/public/local                          [public; language filter]
        timeline::public_local(),
        // GET /api/v1/streaming/public                                [public; language filter]
        timeline::public(),
        // GET /api/v1/streaming/direct                                [private; *no* filter]
        timeline::direct(),
        // GET /api/v1/streaming/hashtag?tag=:hashtag                  [public; no filter]
        timeline::hashtag(),
        // GET /api/v1/streaming/hashtag/local?tag=:hashtag            [public; no filter]
        timeline::hashtag_local(),
        // GET /api/v1/streaming/list?list=:list_id                    [private; no filter]
        timeline::list()
    )
    .untuple_one()
    .and(warp::sse())
    .and(warp::any().map(move || redis_updates.new_copy()))
    .map(
        |timeline: String, user: User, sse: warp::sse::Sse, mut event_stream: StreamManager| {
            dbg!(&event_stream);
            event_stream.add(&timeline, &user);
            event_stream.set_user(user.clone());
            sse.reply(warp::sse::keep(
                event_stream.filter_map(move |item| {
                    let payload = item["payload"].clone();
                    let event = item["event"].clone().to_string();
                    Some((warp::sse::event(event), warp::sse::data(payload)))
                }),
                None,
            ))
        },
    )
    .with(warp::reply::with::header("Connection", "keep-alive"))
    .recover(error::handle_errors);

    let redis_updates_ws = StreamManager::new(Receiver::new());
    let websocket = path!("api" / "v1" / "streaming")
        .and(Scope::Public.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Public))
        .and(warp::query())
        .and(query::Media::to_filter())
        .and(query::Hashtag::to_filter())
        .and(query::List::to_filter())
        .and(warp::ws2())
        .and(warp::any().map(move || redis_updates_ws.new_copy()))
        .and_then(
            |mut user: User,
             q: query::Stream,
             m: query::Media,
             h: query::Hashtag,
             l: query::List,
             ws: warp::ws::Ws2,
             mut stream: StreamManager| {
                let unauthorized = Err(warp::reject::custom("Error: Invalid Access Token"));
                let timeline = match q.stream.as_ref() {
                    // Public endpoints:
                    tl @ "public" | tl @ "public:local" if m.is_truthy() => format!("{}:media", tl),
                    tl @ "public:media" | tl @ "public:local:media" => tl.to_string(),
                    tl @ "public" | tl @ "public:local" => tl.to_string(),
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
                    "direct" => "direct".to_string(),
                    // Other endpoints don't exist:
                    _ => return Err(warp::reject::custom("Error: Nonexistent WebSocket query")),
                };

                stream.add(&timeline, &user);
                stream.set_user(user);
                Ok(ws.on_upgrade(move |socket| ws::handle_ws(socket, stream)))
            },
        );

    warp::serve(websocket.or(routes)).run(([127, 0, 0, 1], 4000));
}
