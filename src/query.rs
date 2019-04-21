use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Media {
    pub only_media: String,
}
#[derive(Deserialize)]
pub struct Hashtag {
    pub tag: String,
}
#[derive(Deserialize)]
pub struct List {
    pub list: i64,
}
#[derive(Deserialize)]
pub struct Auth {
    pub access_token: String,
}
