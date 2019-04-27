mod error;
mod pubsub;
mod query;
mod user;
mod utils;
use futures::stream::Stream;
use futures::{Async, Poll};
use pubsub::PubSub;
use serde_json::Value;
use std::io::Error;
use user::{Filter, Scope, User};
use warp::{path, Filter as WarpFilter};

fn main() {
    pretty_env_logger::init();

    // GET /api/v1/streaming/user                                  [private; language filter]
    let user_timeline = path!("api" / "v1" / "streaming" / "user")
        .and(path::end())
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .map(|user: User| (user.id.to_string(), user));

    // GET /api/v1/streaming/user/notification                     [private; notification filter]
    let user_timeline_notifications = path!("api" / "v1" / "streaming" / "user" / "notification")
        .and(path::end())
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .map(|user: User| (user.id.to_string(), user.with_notification_filter()));

    // GET /api/v1/streaming/public                                [public; language filter]
    let public_timeline = path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .map(|user: User| ("public".into(), user.with_language_filter()));

    // GET /api/v1/streaming/public?only_media=true                [public; language filter]
    let public_timeline_media = path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .and(warp::query())
        .map(|user: User, q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => ("public:media".into(), user.with_language_filter()),
            _ => ("public".into(), user.with_language_filter()),
        });

    // GET /api/v1/streaming/public/local                          [public; language filter]
    let local_timeline = path!("api" / "v1" / "streaming" / "public" / "local")
        .and(path::end())
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .map(|user: User| ("public:local".into(), user.with_language_filter()));

    // GET /api/v1/streaming/public/local?only_media=true          [public; language filter]
    let local_timeline_media = path!("api" / "v1" / "streaming" / "public" / "local")
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .and(warp::query())
        .and(path::end())
        .map(|user: User, q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => ("public:local:media".into(), user.with_language_filter()),
            _ => ("public:local".into(), user.with_language_filter()),
        });

    // GET /api/v1/streaming/direct                                [private; *no* filter]
    let direct_timeline = path!("api" / "v1" / "streaming" / "direct")
        .and(path::end())
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .map(|user: User| (format!("direct:{}", user.id), user.with_no_filter()));

    // GET /api/v1/streaming/hashtag?tag=:hashtag                  [public; no filter]
    let hashtag_timeline = path!("api" / "v1" / "streaming" / "hashtag")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| {
            dbg!(&q);
            (format!("hashtag:{}", q.tag), User::public())
        });

    // GET /api/v1/streaming/hashtag/local?tag=:hashtag            [public; no filter]
    let hashtag_timeline_local = path!("api" / "v1" / "streaming" / "hashtag" / "local")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| (format!("hashtag:{}:local", q.tag), User::public()));

    // GET /api/v1/streaming/list?list=:list_id                    [private; no filter]
    let list_timeline = path!("api" / "v1" / "streaming" / "list")
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .and(warp::query())
        .and_then(|user: User, q: query::List| (user.is_authorized_for_list(q.list), Ok(user)))
        .untuple_one()
        .and(path::end())
        .map(|list: i64, user: User| (format!("list:{}", list), user.with_no_filter()));
    let event_stream = RedisStream::new();
    let event_stream = warp::any().map(move || event_stream.clone());
    let routes = or!(
        user_timeline,
        user_timeline_notifications,
        public_timeline_media,
        public_timeline,
        local_timeline_media,
        local_timeline,
        direct_timeline,
        hashtag_timeline,
        hashtag_timeline_local,
        list_timeline
    )
    .untuple_one()
    .and(warp::sse())
    .and(event_stream)
    .map(
        |timeline: String, user: User, sse: warp::sse::Sse, mut event_stream: RedisStream| {
            event_stream.add(timeline.clone(), user);
            sse.reply(warp::sse::keep(
                event_stream.filter_map(move |item| {
                    println!("ding");
                    Some((warp::sse::event("event"), warp::sse::data(item.to_string())))
                }),
                None,
            ))
        },
    )
    .with(warp::reply::with::header("Connection", "keep-alive"))
    .recover(error::handle_errors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Clone)]
struct RedisStream {
    recv: Arc<Mutex<HashMap<String, pubsub::Receiver>>>,
    current_stream: String,
}
impl RedisStream {
    fn new() -> Self {
        let recv = Arc::new(Mutex::new(HashMap::new()));
        Self {
            recv,
            current_stream: "".to_string(),
        }
    }

    fn add(&mut self, timeline: String, user: User) -> &Self {
        let mut hash_map_of_streams = self.recv.lock().unwrap();
        if !hash_map_of_streams.contains_key(&timeline) {
            println!(
                "First time encountering `{}`, saving it to the HashMap",
                &timeline
            );
            hash_map_of_streams.insert(timeline.clone(), PubSub::from(timeline.clone(), user));
        } else {
            println!(
                "HashMap already contains `{}`, returning unmodified HashMap",
                &timeline
            );
        }
        self.current_stream = timeline;
        self
    }
}
impl Stream for RedisStream {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        println!("polling Interval");
        let mut hash_map_of_streams = self.recv.lock().unwrap();
        let target_stream = self.current_stream.clone();
        let stream = hash_map_of_streams.get_mut(&target_stream).unwrap();
        match stream.poll() {
            Ok(Async::Ready(Some(value))) => Ok(Async::Ready(Some(value))),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
