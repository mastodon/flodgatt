use crate::Id;
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mention {
    pub id: Id,
    username: String,
    acct: String,
    url: String,
}
