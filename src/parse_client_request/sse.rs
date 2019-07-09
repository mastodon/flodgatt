//! Filters for all the endpoints accessible for Server Sent Event updates
use super::{
    query,
    user::{Filter::*, Scope, User},
};
use crate::{config::CustomError, user_from_path};
use warp::{filters::BoxedFilter, path, Filter};

#[allow(dead_code)]
type TimelineUser = ((String, User),);

pub enum Request {}

impl Request {
    /// GET /api/v1/streaming/user
    pub fn user() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "user", Scope::Private)
            .map(|user: User| (user.id.to_string(), user))
            .boxed()
    }

    /// GET /api/v1/streaming/user/notification
    ///
    ///
    /// **NOTE**: This endpoint is not included in the [public API docs](https://docs.joinmastodon.org/api/streaming/#get-api-v1-streaming-public-local).  But it was present in the JavaScript implementation, so has been included here.  Should it be publicly documented?
    pub fn user_notifications() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "user" / "notification", Scope::Private)
            .map(|user: User| (user.id.to_string(), user.set_filter(Notification)))
            .boxed()
    }

    /// GET /api/v1/streaming/public
    pub fn public() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "public", Scope::Public)
            .map(|user: User| ("public".to_owned(), user.set_filter(Language)))
            .boxed()
    }

    /// GET /api/v1/streaming/public?only_media=true
    pub fn public_media() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "public", Scope::Public)
            .and(warp::query())
            .map(|user: User, q: query::Media| match q.only_media.as_ref() {
                "1" | "true" => ("public:media".to_owned(), user.set_filter(Language)),
                _ => ("public".to_owned(), user.set_filter(Language)),
            })
            .boxed()
    }

    /// GET /api/v1/streaming/public/local
    pub fn public_local() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "public" / "local", Scope::Public)
            .map(|user: User| ("public:local".to_owned(), user.set_filter(Language)))
            .boxed()
    }

    /// GET /api/v1/streaming/public/local?only_media=true
    pub fn public_local_media() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "public" / "local", Scope::Public)
            .and(warp::query())
            .map(|user: User, q: query::Media| match q.only_media.as_ref() {
                "1" | "true" => ("public:local:media".to_owned(), user.set_filter(Language)),
                _ => ("public:local".to_owned(), user.set_filter(Language)),
            })
            .boxed()
    }

    /// GET /api/v1/streaming/direct
    pub fn direct() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "direct", Scope::Private)
            .map(|user: User| (format!("direct:{}", user.id), user.set_filter(NoFilter)))
            .boxed()
    }

    /// GET /api/v1/streaming/hashtag?tag=:hashtag
    pub fn hashtag() -> BoxedFilter<TimelineUser> {
        path!("api" / "v1" / "streaming" / "hashtag")
            .and(warp::query())
            .map(|q: query::Hashtag| (format!("hashtag:{}", q.tag), User::public()))
            .boxed()
    }

    /// GET /api/v1/streaming/hashtag/local?tag=:hashtag
    pub fn hashtag_local() -> BoxedFilter<TimelineUser> {
        path!("api" / "v1" / "streaming" / "hashtag" / "local")
            .and(warp::query())
            .map(|q: query::Hashtag| (format!("hashtag:{}:local", q.tag), User::public()))
            .boxed()
    }

    /// GET /api/v1/streaming/list?list=:list_id
    pub fn list() -> BoxedFilter<TimelineUser> {
        user_from_path!("streaming" / "list", Scope::Private)
            .and(warp::query())
            .and_then(|user: User, q: query::List| {
                if user.owns_list(q.list) {
                    (Ok(q.list), Ok(user))
                } else {
                    (Err(CustomError::unauthorized_list()), Ok(user))
                }
            })
            .untuple_one()
            .map(|list: i64, user: User| (format!("list:{}", list), user.set_filter(NoFilter)))
            .boxed()
    }
}
