use super::super::emoji::Emoji;
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Poll {
    pub(crate) id: String,
    pub(crate) expires_at: String,
    pub(crate) expired: bool,
    pub(crate) multiple: bool,
    pub(crate) votes_count: i64,
    pub(crate) voters_count: Option<i64>,
    pub(crate) voted: Option<bool>,
    pub(crate) own_votes: Option<Vec<i64>>,
    pub(crate) options: Vec<PollOptions>,
    pub(crate) emojis: Vec<Emoji>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct PollOptions {
    pub(crate) title: String,
    pub(crate) votes_count: Option<i32>,
}
