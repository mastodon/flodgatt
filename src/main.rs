mod error;
mod pubsub;
mod query;
mod user;
mod utils;
use futures::stream::Stream;
use pretty_env_logger;
use pubsub::stream_from;
use user::{Filter, Scope, User};
use warp::{path, Filter as WarpFilter};

fn main() {
    pretty_env_logger::init();

    // GET /api/v1/streaming/user                                  [private; language filter]
    let user_timeline = path!("api" / "v1" / "streaming" / "user")
        .and(path::end())
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .map(|user: User| stream_from(user.id.to_string(), user));

    // GET /api/v1/streaming/user/notification                     [private; notification filter]
    let user_timeline_notifications = path!("api" / "v1" / "streaming" / "user" / "notification")
        .and(path::end())
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .map(|user: User| stream_from(user.id.to_string(), user.with_notification_filter()));

    // GET /api/v1/streaming/public                                [public; language filter]
    let public_timeline = path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .map(|user: User| stream_from("public".into(), user.with_language_filter()));

    // GET /api/v1/streaming/public?only_media=true                [public; language filter]
    let public_timeline_media = path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .and(warp::query())
        .map(|user: User, q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => stream_from("public:media".into(), user.with_language_filter()),
            _ => stream_from("public".into(), user.with_language_filter()),
        });

    // GET /api/v1/streaming/public/local                          [public; language filter]
    let local_timeline = path!("api" / "v1" / "streaming" / "public" / "local")
        .and(path::end())
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .map(|user: User| stream_from("public:local".into(), user.with_language_filter()));

    // GET /api/v1/streaming/public/local?only_media=true          [public; language filter]
    let local_timeline_media = path!("api" / "v1" / "streaming" / "public" / "local")
        .and(user::get_access_token(user::Scope::Public))
        .and_then(|token| user::get_account(token, Scope::Public))
        .and(warp::query())
        .and(path::end())
        .map(|user: User, q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => stream_from("public:local:media".into(), user.with_language_filter()),
            _ => stream_from("public:local".into(), user.with_language_filter()),
        });

    // GET /api/v1/streaming/direct                                [private; *no* filter]
    let direct_timeline = path!("api" / "v1" / "streaming" / "direct")
        .and(path::end())
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .map(|user: User| stream_from(format!("direct:{}", user.id), user.with_no_filter()));

    // GET /api/v1/streaming/hashtag?tag=:hashtag                  [public; no filter]
    let hashtag_timeline = path!("api" / "v1" / "streaming" / "hashtag")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| stream_from(format!("hashtag:{}", q.tag), User::public()));

    // GET /api/v1/streaming/hashtag/local?tag=:hashtag            [public; no filter]
    let hashtag_timeline_local = path!("api" / "v1" / "streaming" / "hashtag" / "local")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| stream_from(format!("hashtag:{}:local", q.tag), User::public()));

    // GET /api/v1/streaming/list?list=:list_id                    [private; no filter]
    let list_timeline = path!("api" / "v1" / "streaming" / "list")
        .and(user::get_access_token(Scope::Private))
        .and_then(|token| user::get_account(token, Scope::Private))
        .and(warp::query())
        .and_then(|user: User, q: query::List| user.is_authorized_for_list(q.list))
        .untuple_one()
        .and(path::end())
        .map(|list: i64, user: User| stream_from(format!("list:{}", list), user.with_no_filter()));

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
        let user = event_stream.user.clone();
        sse.reply(warp::sse::keep(
            event_stream.filter_map(move |item| {
                let payload = item["payload"].clone();
                let event = item["event"].to_string().clone();
                let toot_lang = item["language"].to_string().clone();

                println!("ding");

                match &user.filter {
                    Filter::Notification if event != "notification" => None,
                    Filter::Language if !user.langs.contains(&toot_lang) => None,
                    _ => Some((warp::sse::event(event), warp::sse::data(payload))),
                }
            }),
            None,
        ))
    })
    .with(warp::reply::with::header("Connection", "keep-alive"))
    .recover(error::handle_errors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
