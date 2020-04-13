use super::{emoji::Emoji, id::Id, visibility::Visibility};
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(super) struct Account {
    pub id: Id,
    username: String,
    pub acct: String,
    url: String,
    display_name: String,
    note: String,
    avatar: String,
    avatar_static: String,
    header: String,
    header_static: String,
    locked: bool,
    emojis: Vec<Emoji>,
    discoverable: Option<bool>, // Shouldn't be option?
    created_at: String,
    statuses_count: i64,
    followers_count: i64,
    following_count: i64,
    moved: Option<String>,
    fields: Option<Vec<Field>>,
    bot: Option<bool>,
    source: Option<Source>,
    group: Option<bool>,            // undocumented
    last_status_at: Option<String>, // undocumented
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Field {
    name: String,
    value: String,
    verified_at: Option<String>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Source {
    note: String,
    fields: Vec<Field>,
    privacy: Option<Visibility>,
    sensitive: bool,
    language: String,
    follow_requests_count: i64,
}
