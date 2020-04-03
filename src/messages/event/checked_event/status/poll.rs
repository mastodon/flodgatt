use super::super::emoji::Emoji;
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(super) struct Poll {
    id: String,
    expires_at: String,
    expired: bool,
    multiple: bool,
    votes_count: i64,
    voters_count: Option<i64>,
    voted: Option<bool>,
    own_votes: Option<Vec<i64>>,
    options: Vec<PollOptions>,
    emojis: Vec<Emoji>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct PollOptions {
    title: String,
    votes_count: Option<i32>,
}
