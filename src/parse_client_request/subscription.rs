//! `User` struct and related functionality
// #[cfg(test)]
// mod mock_postgres;
// #[cfg(test)]
// use mock_postgres as postgres;
// #[cfg(not(test))]

use super::postgres::PgPool;
use super::query::Query;
use crate::err::TimelineErr;
use crate::log_fatal;
use lru::LruCache;
use std::collections::HashSet;
use warp::reject::Rejection;

use super::query;
use warp::{filters::BoxedFilter, path, Filter};

/// Helper macro to match on the first of any of the provided filters
macro_rules! any_of {
    ($filter:expr, $($other_filter:expr),*) => {
        $filter$(.or($other_filter).unify())*.boxed()
    };
}
macro_rules! parse_sse_query {
    (path => $start:tt $(/ $next:tt)*
     endpoint => $endpoint:expr) => {
        path!($start $(/ $next)*)
            .and(query::Auth::to_filter())
            .and(query::Media::to_filter())
            .and(query::Hashtag::to_filter())
            .and(query::List::to_filter())
            .map(
                |auth: query::Auth,
                 media: query::Media,
                 hashtag: query::Hashtag,
                 list: query::List| {
                    Query {
                        access_token: auth.access_token,
                        stream: $endpoint.to_string(),
                        media: media.is_truthy(),
                        hashtag: hashtag.tag,
                        list: list.list,
                    }
                 },
            )
            .boxed()
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct Subscription {
    pub timeline: Timeline,
    pub allowed_langs: HashSet<String>,
    pub blocks: Blocks,
    pub hashtag_name: Option<String>,
    pub access_token: Option<String>,
}

impl Default for Subscription {
    fn default() -> Self {
        Self {
            timeline: Timeline(Stream::Unset, Reach::Local, Content::Notification),
            allowed_langs: HashSet::new(),
            blocks: Blocks::default(),
            hashtag_name: None,
            access_token: None,
        }
    }
}

impl Subscription {
    pub fn from_ws_request(pg_pool: PgPool, whitelist_mode: bool) -> BoxedFilter<(Subscription,)> {
        parse_ws_query()
            .and(query::OptionalAccessToken::from_ws_header())
            .and_then(Query::update_access_token)
            .and_then(move |q| Subscription::from_query(q, pg_pool.clone(), whitelist_mode))
            .boxed()
    }

    pub fn from_sse_query(pg_pool: PgPool, whitelist_mode: bool) -> BoxedFilter<(Subscription,)> {
        any_of!(
            parse_sse_query!(
            path => "api" / "v1" / "streaming" / "user" / "notification"
            endpoint => "user:notification" ),
            parse_sse_query!(
            path => "api" / "v1" / "streaming" / "user"
            endpoint => "user"),
            parse_sse_query!(
            path => "api" / "v1" / "streaming" / "public" / "local"
            endpoint => "public:local"),
            parse_sse_query!(
            path => "api" / "v1" / "streaming" / "public"
            endpoint => "public"),
            parse_sse_query!(
            path => "api" / "v1" / "streaming" / "direct"
            endpoint => "direct"),
            parse_sse_query!(path => "api" / "v1" / "streaming" / "hashtag" / "local"
                     endpoint => "hashtag:local"),
            parse_sse_query!(path => "api" / "v1" / "streaming" / "hashtag"
                     endpoint => "hashtag"),
            parse_sse_query!(path => "api" / "v1" / "streaming" / "list"
                endpoint => "list")
        )
        // because SSE requests place their `access_token` in the header instead of in a query
        // parameter, we need to update our Query if the header has a token
        .and(query::OptionalAccessToken::from_sse_header())
        .and_then(Query::update_access_token)
        .and_then(move |q| Subscription::from_query(q, pg_pool.clone(), whitelist_mode))
        .boxed()
    }

    fn from_query(q: Query, pool: PgPool, whitelist_mode: bool) -> Result<Self, Rejection> {
        let user = match q.access_token.clone() {
            Some(token) => pool.clone().select_user(&token)?,
            None if whitelist_mode => Err(warp::reject::custom("Error: Invalid access token"))?,
            None => UserData::public(),
        };
        let timeline = Timeline::from_query_and_user(&q, &user, pool.clone())?;
        let hashtag_name = match timeline {
            Timeline(Stream::Hashtag(_), _, _) => Some(q.hashtag),
            _non_hashtag_timeline => None,
        };

        Ok(Subscription {
            timeline,
            allowed_langs: user.allowed_langs,
            blocks: Blocks {
                blocking_users: pool.clone().select_blocking_users(user.id),
                blocked_users: pool.clone().select_blocked_users(user.id),
                blocked_domains: pool.clone().select_blocked_domains(user.id),
            },
            hashtag_name,
            access_token: q.access_token,
        })
    }
}

fn parse_ws_query() -> BoxedFilter<(Query,)> {
    path!("api" / "v1" / "streaming")
        .and(path::end())
        .and(warp::query())
        .and(query::Auth::to_filter())
        .and(query::Media::to_filter())
        .and(query::Hashtag::to_filter())
        .and(query::List::to_filter())
        .map(
            |stream: query::Stream,
             auth: query::Auth,
             media: query::Media,
             hashtag: query::Hashtag,
             list: query::List| {
                Query {
                    access_token: auth.access_token,
                    stream: stream.stream,
                    media: media.is_truthy(),
                    hashtag: hashtag.tag,
                    list: list.list,
                }
            },
        )
        .boxed()
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub struct Timeline(pub Stream, pub Reach, pub Content);

impl Timeline {
    pub fn empty() -> Self {
        use {Content::*, Reach::*, Stream::*};
        Self(Unset, Local, Notification)
    }

    pub fn to_redis_raw_timeline(&self, hashtag: Option<&String>) -> String {
        use {Content::*, Reach::*, Stream::*};
        match self {
            Timeline(Public, Federated, All) => "timeline:public".into(),
            Timeline(Public, Local, All) => "timeline:public:local".into(),
            Timeline(Public, Federated, Media) => "timeline:public:media".into(),
            Timeline(Public, Local, Media) => "timeline:public:local:media".into(),

            Timeline(Hashtag(id), Federated, All) => format!(
                "timeline:hashtag:{}",
                hashtag.unwrap_or_else(|| log_fatal!("Did not supply a name for hashtag #{}", id))
            ),
            Timeline(Hashtag(id), Local, All) => format!(
                "timeline:hashtag:{}:local",
                hashtag.unwrap_or_else(|| log_fatal!("Did not supply a name for hashtag #{}", id))
            ),
            Timeline(User(id), Federated, All) => format!("timeline:{}", id),
            Timeline(User(id), Federated, Notification) => format!("timeline:{}:notification", id),
            Timeline(List(id), Federated, All) => format!("timeline:list:{}", id),
            Timeline(Direct(id), Federated, All) => format!("timeline:direct:{}", id),
            Timeline(one, _two, _three) => {
                log_fatal!("Supposedly impossible timeline reached: {:?}", one)
            }
        }
    }

    pub fn from_redis_text(
        timeline: &str,
        cache: &mut LruCache<String, i64>,
    ) -> Result<Self, TimelineErr> {
        let mut id_from_tag = |tag: &str| match cache.get(&tag.to_string()) {
            Some(id) => Ok(*id),
            None => Err(TimelineErr::InvalidInput), // TODO more specific
        };

        use {Content::*, Reach::*, Stream::*};
        Ok(match &timeline.split(":").collect::<Vec<&str>>()[..] {
            ["public"] => Timeline(Public, Federated, All),
            ["public", "local"] => Timeline(Public, Local, All),
            ["public", "media"] => Timeline(Public, Federated, Media),
            ["public", "local", "media"] => Timeline(Public, Local, Media),
            ["hashtag", tag] => Timeline(Hashtag(id_from_tag(tag)?), Federated, All),
            ["hashtag", tag, "local"] => Timeline(Hashtag(id_from_tag(tag)?), Local, All),
            [id] => Timeline(User(id.parse().unwrap()), Federated, All),
            [id, "notification"] => Timeline(User(id.parse().unwrap()), Federated, Notification),
            ["list", id] => Timeline(List(id.parse().unwrap()), Federated, All),
            ["direct", id] => Timeline(Direct(id.parse().unwrap()), Federated, All),
            // Other endpoints don't exist:
            [..] => Err(TimelineErr::InvalidInput)?,
        })
    }

    fn from_query_and_user(q: &Query, user: &UserData, pool: PgPool) -> Result<Self, Rejection> {
        use {warp::reject::custom, Content::*, Reach::*, Scope::*, Stream::*};
        let id_from_hashtag = || pool.clone().select_hashtag_id(&q.hashtag);
        let user_owns_list = || pool.clone().user_owns_list(user.id, q.list);

        Ok(match q.stream.as_ref() {
            "public" => match q.media {
                true => Timeline(Public, Federated, Media),
                false => Timeline(Public, Federated, All),
            },
            "public:local" => match q.media {
                true => Timeline(Public, Local, Media),
                false => Timeline(Public, Local, All),
            },
            "public:media" => Timeline(Public, Federated, Media),
            "public:local:media" => Timeline(Public, Local, Media),

            "hashtag" => Timeline(Hashtag(id_from_hashtag()?), Federated, All),
            "hashtag:local" => Timeline(Hashtag(id_from_hashtag()?), Local, All),
            "user" => match user.scopes.contains(&Statuses) {
                true => Timeline(User(user.id), Federated, All),
                false => Err(custom("Error: Missing access token"))?,
            },
            "user:notification" => match user.scopes.contains(&Statuses) {
                true => Timeline(User(user.id), Federated, Notification),
                false => Err(custom("Error: Missing access token"))?,
            },
            "list" => match user.scopes.contains(&Lists) && user_owns_list() {
                true => Timeline(List(q.list), Federated, All),
                false => Err(warp::reject::custom("Error: Missing access token"))?,
            },
            "direct" => match user.scopes.contains(&Statuses) {
                true => Timeline(Direct(user.id), Federated, All),
                false => Err(custom("Error: Missing access token"))?,
            },
            other => {
                log::warn!("Request for nonexistent endpoint: `{}`", other);
                Err(custom("Error: Nonexistent endpoint"))?
            }
        })
    }
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum Stream {
    User(i64),
    List(i64),
    Direct(i64),
    Hashtag(i64),
    Public,
    Unset,
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum Reach {
    Local,
    Federated,
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum Content {
    All,
    Media,
    Notification,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Read,
    Statuses,
    Notifications,
    Lists,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Blocks {
    pub blocked_domains: HashSet<String>,
    pub blocked_users: HashSet<i64>,
    pub blocking_users: HashSet<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserData {
    pub id: i64,
    pub allowed_langs: HashSet<String>,
    pub scopes: HashSet<Scope>,
}

impl UserData {
    fn public() -> Self {
        Self {
            id: -1,
            allowed_langs: HashSet::new(),
            scopes: HashSet::new(),
        }
    }
}
