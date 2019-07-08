//! Validate query prarams with type checking
use serde_derive::Deserialize;
use warp::filters::BoxedFilter;
use warp::Filter as WarpFilter;

macro_rules! query {
    ($name:tt => $parameter:tt:$type:tt) => {
        #[derive(Deserialize, Debug, Default)]
        pub struct $name {
            pub $parameter: $type,
        }
        impl $name {
            pub fn to_filter() -> BoxedFilter<(Self,)> {
                warp::query()
                    .or(warp::any().map(Self::default))
                    .unify()
                    .boxed()
            }
        }
    };
}
query!(Media => only_media:String);
impl Media {
    pub fn is_truthy(&self) -> bool {
        self.only_media == "true" || self.only_media == "1"
    }
}
query!(Hashtag => tag: String);
query!(List => list: i64);
query!(Auth => access_token: String);
query!(Stream => stream: String);
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
