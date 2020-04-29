use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Tag {
    pub(super) name: String,
    pub(super) url: String,
    pub(super) history: Option<Vec<History>>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct History {
    pub(super) day: String,
    pub(super) uses: String,
    pub(super) accounts: String,
}
