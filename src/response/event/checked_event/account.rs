use super::{emoji::Emoji, visibility::Visibility};
use crate::Id;
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Account {
    pub id: Id,
    pub(super) username: String,
    pub acct: String,
    pub(super) url: String,
    pub(super) display_name: String,
    pub(super) note: String,
    pub(super) avatar: String,
    pub(super) avatar_static: String,
    pub(super) header: String,
    pub(super) header_static: String,
    pub(super) locked: bool,
    pub(super) emojis: Vec<Emoji>,
    pub(super) discoverable: Option<bool>, // Shouldn't be option?
    pub(super) created_at: String,
    pub(super) statuses_count: i64,
    pub(super) followers_count: i64,
    pub(super) following_count: i64,
    pub(super) moved: Option<String>,
    pub(super) fields: Option<Vec<Field>>,
    pub(super) bot: Option<bool>,
    pub(super) source: Option<Source>,
    pub(super) group: Option<bool>,            // undocumented
    pub(super) last_status_at: Option<String>, // undocumented
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Field {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) verified_at: Option<String>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Source {
    pub(super) note: String,
    pub(super) fields: Vec<Field>,
    pub(super) privacy: Option<Visibility>,
    pub(super) sensitive: bool,
    pub(super) language: String,
    pub(super) follow_requests_count: i64,
}
