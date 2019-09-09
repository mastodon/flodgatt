//! Validate query prarams with type checking
use serde_derive::Deserialize;
use warp::filters::BoxedFilter;
use warp::Filter as WarpFilter;

#[derive(Debug)]
pub struct Query {
    pub access_token: Option<String>,
    pub stream: String,
    pub media: bool,
    pub hashtag: String,
    pub list: i64,
}

impl Query {
    pub fn update_access_token(
        self,
        token: Option<String>,
    ) -> Result<Self, warp::reject::Rejection> {
        match token {
            Some(token) => Ok(Self {
                access_token: Some(token),
                ..self
            }),
            None => Ok(self),
        }
    }
}

macro_rules! make_query_type {
    ($name:tt => $parameter:tt:$type:ty) => {
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
make_query_type!(Media => only_media:String);
impl Media {
    pub fn is_truthy(&self) -> bool {
        self.only_media == "true" || self.only_media == "1"
    }
}
make_query_type!(Hashtag => tag: String);
make_query_type!(List => list: i64);
make_query_type!(Auth => access_token: Option<String>);
make_query_type!(Stream => stream: String);
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

pub struct OptionalAccessToken;

impl OptionalAccessToken {
    pub fn from_header() -> warp::filters::BoxedFilter<(Option<String>,)> {
        let from_header = warp::header::header::<String>("authorization").map(|auth: String| {
            match auth.split(' ').nth(1) {
                Some(s) => Some(s.to_string()),
                None => None,
            }
        });
        let no_token = warp::any().map(|| None);

        from_header.or(no_token).unify().boxed()
    }
}
