use super::id::Id;
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(super) struct Mention {
    pub id: Id,
    username: String,
    acct: String,
    url: String,
}
