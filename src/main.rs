mod auth;
mod error;
mod pubsub;
mod query;
mod utils;
use futures::stream::Stream;
use pretty_env_logger;
use warp::{path, Filter};

fn main() {
    pretty_env_logger::init();

    // GET /api/v1/streaming/user
    let user_timeline = path!("api" / "v1" / "streaming" / "user")
        .and(path::end())
        .and(auth::get_token())
        .and_then(auth::get_account_id_from_token)
        .map(|account_id: i64| pubsub::stream_from(account_id.to_string()));

    // GET /api/v1/streaming/user/notification
    let user_timeline_notifications = path!("api" / "v1" / "streaming" / "user" / "notification")
        .and(path::end())
        .and(auth::get_token())
        .and_then(auth::get_account_id_from_token)
        .map(|account_id: i64| {
            let full_stream = pubsub::stream_from(account_id.to_string());
            // TODO: filter stream to just have notifications
            full_stream
        });

    // GET /api/v1/streaming/public
    let public_timeline = path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .map(|| pubsub::stream_from("public".to_string()));

    // GET /api/v1/streaming/public?only_media=true
    let public_timeline_media = path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .and(warp::query())
        .map(|q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => pubsub::stream_from("public:media".to_string()),
            _ => pubsub::stream_from("public".to_string()),
        });

    // GET /api/v1/streaming/public/local
    let local_timeline = path!("api" / "v1" / "streaming" / "public" / "local")
        .and(path::end())
        .map(|| pubsub::stream_from("public:local".to_string()));

    // GET /api/v1/streaming/public/local?only_media=true
    let local_timeline_media = path!("api" / "v1" / "streaming" / "public" / "local")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => pubsub::stream_from("public:local:media".to_string()),
            _ => pubsub::stream_from("public:local".to_string()),
        });

    // GET /api/v1/streaming/direct
    let direct_timeline = path!("api" / "v1" / "streaming" / "direct")
        .and(path::end())
        .and(auth::get_token())
        .and_then(auth::get_account_id_from_token)
        .map(|account_id: i64| pubsub::stream_from(format!("direct:{}", account_id)));

    // GET /api/v1/streaming/hashtag?tag=:hashtag
    let hashtag_timeline = path!("api" / "v1" / "streaming" / "hashtag")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| pubsub::stream_from(format!("hashtag:{}", q.tag)));

    // GET /api/v1/streaming/hashtag/local?tag=:hashtag
    let hashtag_timeline_local = path!("api" / "v1" / "streaming" / "hashtag" / "local")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| pubsub::stream_from(format!("hashtag:{}:local", q.tag)));

    // GET /api/v1/streaming/list?list=:list_id
    let list_timeline = path!("api" / "v1" / "streaming" / "list")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::List| pubsub::stream_from(format!("list:{}", q.list)));

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
    .and_then(|event_stream| event_stream)
    .and(warp::sse())
    .map(|event_stream: pubsub::Receiver, sse: warp::sse::Sse| {
        sse.reply(warp::sse::keep(
            event_stream.map(|item| {
                let payload = item["payload"].clone();
                let event = item["event"].clone();
                (warp::sse::event(event), warp::sse::data(payload))
            }),
            None,
        ))
    })
    .recover(error::handle_errors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
