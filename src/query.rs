//! Validate query prarams with type checking
use serde_derive::Deserialize;
use warp::filters::BoxedFilter;
use warp::Filter as WarpFilter;

#[derive(Deserialize, Debug, Default)]
pub struct Media {
    pub only_media: String,
}
impl Media {
    pub fn to_filter() -> BoxedFilter<(Self,)> {
        warp::query()
            .or(warp::any().map(Self::default))
            .unify()
            .boxed()
    }
    pub fn is_truthy(&self) -> bool {
        self.only_media == "true" || self.only_media == "1"
    }
}
#[derive(Deserialize, Debug, Default)]
pub struct Hashtag {
    pub tag: String,
}
impl Hashtag {
    pub fn to_filter() -> BoxedFilter<(Self,)> {
        warp::query()
            .or(warp::any().map(Self::default))
            .unify()
            .boxed()
    }
}
#[derive(Deserialize, Debug, Default)]
pub struct List {
    pub list: i64,
}
impl List {
    pub fn to_filter() -> BoxedFilter<(Self,)> {
        warp::query()
            .or(warp::any().map(Self::default))
            .unify()
            .boxed()
    }
}
#[derive(Deserialize, Debug)]
pub struct Auth {
    pub access_token: String,
}
#[derive(Deserialize, Debug)]
pub struct Stream {
    pub stream: String,
}
impl ToString for Stream {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub fn optional_media_query() -> BoxedFilter<(Media,)> {
    warp::query()
        .or(warp::any().map(|| Media {
            only_media: "false".to_owned(),
        }))
        .unify()
        .boxed()
}
