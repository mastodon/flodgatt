//! Parse the client request and return a Subscription
mod postgres;
mod query;
pub mod timeline;

mod subscription;

pub use self::postgres::PgPool;
// TODO consider whether we can remove `Stream` from public API
pub use subscription::{Blocks, Subscription};
pub use timeline::{Content, Reach, Stream, Timeline};

use self::query::Query;
use crate::config;
use warp::{filters::BoxedFilter, path, Filter};

#[cfg(test)]
mod sse_test;
#[cfg(test)]
mod ws_test;

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
    pub fn new(postgres_cfg: config::Postgres, whitelist_mode: bool) -> Self {
        Self {
            pg_conn: PgPool::new(postgres_cfg, whitelist_mode),
        }
    }

    pub fn parse_ws_request(&self) -> BoxedFilter<(Subscription,)> {
        let pg_conn = self.pg_conn.clone();
        parse_ws_query()
            .and(query::OptionalAccessToken::from_ws_header())
            .and_then(Query::update_access_token)
            .and_then(move |q| Subscription::query_postgres(q, pg_conn.clone()))
            .boxed()
    }

    pub fn parse_sse_request(&self) -> BoxedFilter<(Subscription,)> {
        let pg_conn = self.pg_conn.clone();
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
        .and_then(move |q| Subscription::query_postgres(q, pg_conn.clone()))
        .boxed()
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
