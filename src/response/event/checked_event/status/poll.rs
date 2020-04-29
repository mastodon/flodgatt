use super::super::emoji::Emoji;
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Poll {
    pub(super) id: String,
    pub(super) expires_at: String,
    pub(super) expired: bool,
    pub(super) multiple: bool,
    pub(super) votes_count: i64,
    pub(super) voters_count: Option<i64>,
    pub(super) voted: Option<bool>,
    pub(super) own_votes: Option<Vec<i64>>,
    pub(super) options: Vec<PollOptions>,
    pub(super) emojis: Vec<Emoji>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct PollOptions {
    pub(super) title: String,
    pub(super) votes_count: Option<i32>,
}
