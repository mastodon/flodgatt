use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(super) struct Tag {
    name: String,
    url: String,
    history: Option<Vec<History>>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct History {
    day: String,
    uses: String,
    accounts: String,
}
