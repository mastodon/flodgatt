use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Media {
    pub only_media: String,
}
#[derive(Deserialize, Debug)]
pub struct Hashtag {
    pub tag: String,
}
#[derive(Deserialize, Debug)]
pub struct List {
    pub list: i64,
}
#[derive(Deserialize, Debug)]
pub struct Auth {
    pub access_token: String,
}
