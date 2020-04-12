//! Parse the client request and return a Subscription
mod postgres;
mod query;

mod subscription;

pub use self::postgres::PgPool;
// TODO consider whether we can remove `Stream` from public API
pub use subscription::{Blocks, Stream, Subscription, Timeline};
pub use subscription::{Content, Reach};

use self::query::Query;
use crate::config;
use warp::{filters::BoxedFilter, path, reject::Rejection, Filter};

#[cfg(test)]
mod sse_test;
#[cfg(test)]
mod ws_test;

pub struct Handler {
    pg_conn: PgPool,
}

impl Handler {
    pub fn new(postgres_cfg: config::Postgres, whitelist_mode: bool) -> Self {
        Self {
            pg_conn: PgPool::new(postgres_cfg, whitelist_mode),
        }
    }

    pub fn from_ws_request(&self) -> BoxedFilter<(Subscription,)> {
        let pg_conn = self.pg_conn.clone();
        parse_ws_query()
            .and(query::OptionalAccessToken::from_ws_header())
            .and_then(Query::update_access_token)
            .and_then(move |q| Subscription::from_query(q, pg_conn.clone()))
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
