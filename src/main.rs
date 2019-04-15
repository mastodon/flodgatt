mod pubsub;
mod query;
use futures::stream::Stream;
use warp::{path, Filter};

fn main() {
    use warp::path;
    let base = path!("api" / "v1" / "streaming");

    // GET /api/v1/streaming/user
    let user_timeline = base
        .and(path("user"))
        .and(path::end())
        // TODO get user id from postgress
        .map(|| pubsub::stream_from("1".to_string()));

    // GET /api/v1/streaming/user/notification
    let user_timeline_notifications = base
        .and(path!("user" / "notification"))
        .and(path::end())
        // TODO get user id from postgress
        .map(|| {
            let full_stream = pubsub::stream_from("1".to_string());
            // TODO: filter stream to just have notifications
            full_stream
        });

    // GET /api/v1/streaming/public
    let public_timeline = base
        .and(path("public"))
        .and(path::end())
        .map(|| pubsub::stream_from("public".to_string()));

    // GET /api/v1/streaming/public?only_media=true
    let public_timeline_media = base
        .and(path("public"))
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Media| {
            if q.only_media == "1" || q.only_media == "true" {
                pubsub::stream_from("public:media".to_string())
            } else {
                pubsub::stream_from("public".to_string())
            }
        });

    // GET /api/v1/streaming/public/local
    let local_timeline = base
        .and(path!("public" / "local"))
        .and(path::end())
        .map(|| pubsub::stream_from("public:local".to_string()));

    // GET /api/v1/streaming/public/local?only_media=true
    let local_timeline_media = base
        .and(path!("public" / "local"))
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Media| {
            if q.only_media == "1" || q.only_media == "true" {
                pubsub::stream_from("public:local:media".to_string())
            } else {
                pubsub::stream_from("public:local".to_string())
            }
        });

    // GET /api/v1/streaming/direct
    let direct_timeline = base
        .and(path("direct"))
        .and(path::end())
        // TODO get user id from postgress
        .map(|| pubsub::stream_from("direct:1".to_string()));

    // GET /api/v1/streaming/hashtag?tag=:hashtag
    let hashtag_timeline = base
        .and(path("hashtag"))
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| pubsub::stream_from(format!("hashtag:{}", q.tag)));

    // GET /api/v1/streaming/hashtag/local?tag=:hashtag
    let hashtag_timeline_local = base
        .and(path!("hashtag" / "local"))
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| pubsub::stream_from(format!("hashtag:{}:local", q.tag)));

    // GET /api/v1/streaming/list?list=:list_id
    let list_timeline = base
        .and(path("list"))
        .and(warp::query())
        .and(path::end())
        .map(|q: query::List| pubsub::stream_from(format!("list:{}", q.list)));

    let routes = user_timeline
        .or(user_timeline_notifications)
        .unify()
        .or(public_timeline_media)
        .unify()
        .or(public_timeline)
        .unify()
        .or(local_timeline_media)
        .unify()
        .or(local_timeline)
        .unify()
        .or(direct_timeline)
        .unify()
        .or(hashtag_timeline)
        .unify()
        .or(hashtag_timeline_local)
        .unify()
        .or(list_timeline)
        .unify()
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
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
