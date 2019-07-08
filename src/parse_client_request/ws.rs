//! Filters for the WebSocket endpoint
use super::{
    query,
    user::{Scope, User},
};
use crate::user_from_path;
use warp::{filters::BoxedFilter, path, Filter};

/// WebSocket filters
pub fn extract_user_and_query() -> BoxedFilter<(User, Query, warp::ws::Ws2)> {
    user_from_path!("streaming", Scope::Public)
        .and(warp::query())
        .and(query::Media::to_filter())
        .and(query::Hashtag::to_filter())
        .and(query::List::to_filter())
        .and(warp::ws2())
        .map(
            |user: User,
             stream: query::Stream,
             media: query::Media,
             hashtag: query::Hashtag,
             list: query::List,
             ws: warp::ws::Ws2| {
                let query = Query {
                    stream: stream.stream,
                    media: media.is_truthy(),
                    hashtag: hashtag.tag,
                    list: list.list,
                };
                (user, query, ws)
            },
        )
        .untuple_one()
        .boxed()
}

#[derive(Debug)]
pub struct Query {
    pub stream: String,
    pub media: bool,
    pub hashtag: String,
    pub list: i64,
}
