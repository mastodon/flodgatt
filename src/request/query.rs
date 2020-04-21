//! Validate query prarams with type checking
use serde_derive::Deserialize;
use warp::filters::BoxedFilter;
use warp::Filter as WarpFilter;

#[derive(Debug)]
pub(crate) struct Query {
    pub(crate) access_token: Option<String>,
    pub(crate) stream: String,
    pub(crate) media: bool,
    pub(crate) hashtag: String,
    pub(crate) list: i64,
}

impl Query {
    pub(crate) fn update_access_token(
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
    (Stream => $parameter:tt:$type:ty) => {
        #[derive(Deserialize, Debug, Default)]
        pub(crate) struct Stream {
            pub(crate) $parameter: $type,
        }
    };
    ($name:tt => $parameter:tt:$type:ty) => {
        #[derive(Deserialize, Debug, Default)]
        pub(crate) struct $name {
            pub(crate) $parameter: $type,
        }
        impl $name {
            pub(crate) fn to_filter() -> BoxedFilter<(Self,)> {
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
    pub(crate) fn is_truthy(&self) -> bool {
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

pub(super) struct OptionalAccessToken;

impl OptionalAccessToken {
    pub(super) fn from_sse_header() -> warp::filters::BoxedFilter<(Option<String>,)> {
        let from_header = warp::header::header::<String>("authorization").map(|auth: String| {
            match auth.split(' ').nth(1) {
                Some(s) => Some(s.to_string()),
                None => None,
            }
        });
        let no_token = warp::any().map(|| None);

        from_header.or(no_token).unify().boxed()
    }
    pub(super) fn from_ws_header() -> warp::filters::BoxedFilter<(Option<String>,)> {
        let from_header = warp::header::header::<String>("Sec-Websocket-Protocol").map(Some);
        let no_token = warp::any().map(|| None);

        from_header.or(no_token).unify().boxed()
    }
}
