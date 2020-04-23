use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Tag {
    name: String,
    url: String,
    history: Option<Vec<History>>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct History {
    day: String,
    uses: String,
    accounts: String,
}
