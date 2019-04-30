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
            event_stream.add(&timeline, &user);
            sse.reply(warp::sse::keep(
                event_stream.filter_map(move |item| {
                    let payload = item["payload"].clone();
                    let event = item["event"].clone().to_string();
                    let toot_lang = payload["language"].as_str().expect("redis str").to_string();
                    let user_langs = user.langs.clone();

                    match (&user.filter, user_langs) {
                        (Filter::Notification, _) if event != "notification" => None,
                        (Filter::Language, Some(ref langs)) if !langs.contains(&toot_lang) => None,
                        _ => Some((warp::sse::event(event), warp::sse::data(payload))),
                    }
                }),
                None,
            ))
        },
    )
    .with(warp::reply::with::header("Connection", "keep-alive"))
    .recover(error::handle_errors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
