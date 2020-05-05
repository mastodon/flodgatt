use super::{emoji::Emoji, visibility::Visibility};
use crate::Id;
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Account {
    pub id: Id,
    pub(crate) username: String,
    pub acct: String,
    pub(crate) url: String,
    pub(crate) display_name: String,
    pub(crate) note: String,
    pub(crate) avatar: String,
    pub(crate) avatar_static: String,
    pub(crate) header: String,
    pub(crate) header_static: String,
    pub(crate) locked: bool,
    pub(crate) emojis: Vec<Emoji>,
    pub(crate) discoverable: Option<bool>, // Shouldn't be option?
    pub(crate) created_at: String,
    pub(crate) statuses_count: i64,
    pub(crate) followers_count: i64,
    pub(crate) following_count: i64,
    pub(crate) moved: Option<String>,
    pub(crate) fields: Option<Vec<Field>>,
    pub(crate) bot: Option<bool>,
    pub(crate) source: Option<Source>,
    pub(crate) group: Option<bool>,            // undocumented
    pub(crate) last_status_at: Option<String>, // undocumented
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) verified_at: Option<String>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) note: String,
    pub(crate) fields: Vec<Field>,
    pub(crate) privacy: Option<Visibility>,
    pub(crate) sensitive: bool,
    pub(crate) language: String,
    pub(crate) follow_requests_count: i64,
}
