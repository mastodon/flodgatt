//! Parse the client request and return a 'timeline' and a (maybe authenticated) `User`
pub mod query;
pub mod sse;
pub mod user;
pub mod ws;

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
