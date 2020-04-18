//! Parse the client request and return a Subscription
mod postgres;
mod query;
mod timeline;

pub mod err;
mod subscription;

pub(crate) use err::Error;
pub use subscription::Subscription;
pub use timeline::Timeline;
use timeline::{Content, Reach, Stream};

use self::postgres::PgPool;
use self::query::Query;
use crate::config::Postgres;
use warp::filters::BoxedFilter;
use warp::http::StatusCode;
use warp::path;
use warp::{Filter, Rejection};

#[cfg(test)]
mod sse_test;
#[cfg(test)]
mod ws_test;

type Result<T> = std::result::Result<T, err::Error>;

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
            .map(|auth: query::Auth, media: query::Media, hashtag: query::Hashtag, list: query::List| {
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

#[derive(Debug, Clone)]
pub struct Handler {
    pg_conn: PgPool,
}

impl Handler {
    pub fn new(postgres_cfg: &Postgres, whitelist_mode: bool) -> Result<Self> {
        Ok(Self {
            pg_conn: PgPool::new(postgres_cfg, whitelist_mode)?,
        })
    }

    pub fn sse_subscription(&self) -> BoxedFilter<(Subscription,)> {
        let pg_conn = self.pg_conn.clone();
        any_of!(
            parse_sse_query!( path => "api" / "v1" / "streaming" / "user" / "notification"
                              endpoint => "user:notification" ),
            parse_sse_query!( path => "api" / "v1" / "streaming" / "user"
                              endpoint => "user"),
            parse_sse_query!( path => "api" / "v1" / "streaming" / "public" / "local"
                              endpoint => "public:local"),
            parse_sse_query!( path => "api" / "v1" / "streaming" / "public"
                              endpoint => "public"),
            parse_sse_query!( path => "api" / "v1" / "streaming" / "direct"
                              endpoint => "direct"),
            parse_sse_query!( path => "api" / "v1" / "streaming" / "hashtag" / "local"
                              endpoint => "hashtag:local"),
            parse_sse_query!( path => "api" / "v1" / "streaming" / "hashtag"
                              endpoint => "hashtag"),
            parse_sse_query!( path => "api" / "v1" / "streaming" / "list"
                              endpoint => "list")
        )
        // because SSE requests place their `access_token` in the header instead of in a query
        // parameter, we need to update our Query if the header has a token
        .and(query::OptionalAccessToken::from_sse_header())
        .and_then(Query::update_access_token)
        .and_then(move |q| Subscription::query_postgres(q, pg_conn.clone()))
        .boxed()
    }

    pub fn ws_subscription(&self) -> BoxedFilter<(Subscription,)> {
        let pg_conn = self.pg_conn.clone();
        parse_ws_query()
            .and(query::OptionalAccessToken::from_ws_header())
            .and_then(Query::update_access_token)
            .and_then(move |q| Subscription::query_postgres(q, pg_conn.clone()))
            .boxed()
    }

    pub fn health(&self) -> BoxedFilter<()> {
        warp::path!("api" / "v1" / "streaming" / "health").boxed()
    }

    pub fn status(&self) -> BoxedFilter<()> {
        warp::path!("api" / "v1" / "streaming" / "status")
            .and(warp::path::end())
            .boxed()
    }

    pub fn status_per_timeline(&self) -> BoxedFilter<()> {
        warp::path!("api" / "v1" / "streaming" / "status" / "per_timeline").boxed()
    }

    pub fn err(r: Rejection) -> std::result::Result<impl warp::Reply, warp::Rejection> {
        let json_err = match r.cause() {
            Some(text) if text.to_string() == "Missing request header 'authorization'" => {
                warp::reply::json(&"Error: Missing access token".to_string())
            }
            Some(text) => warp::reply::json(&text.to_string()),
            None => warp::reply::json(&"Error: Nonexistant endpoint".to_string()),
        };
        Ok(warp::reply::with_status(json_err, StatusCode::UNAUTHORIZED))
    }
}

fn parse_ws_query() -> BoxedFilter<(Query,)> {
    use query::*;
    path!("api" / "v1" / "streaming")
        .and(path::end())
        .and(warp::query())
        .and(Auth::to_filter())
        .and(Media::to_filter())
        .and(Hashtag::to_filter())
        .and(List::to_filter())
        .map(|s: Stream, a: Auth, m: Media, h: Hashtag, l: List| Query {
            access_token: a.access_token,
            stream: s.stream,
            media: m.is_truthy(),
            hashtag: h.tag,
            list: l.list,
        })
        .boxed()
}
