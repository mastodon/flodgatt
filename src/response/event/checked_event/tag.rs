use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tag {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) history: Option<Vec<History>>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct History {
    pub(crate) day: String,
    pub(crate) uses: String,
    pub(crate) accounts: String,
}
