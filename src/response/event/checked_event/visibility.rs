use serde::{Deserialize, Serialize};

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) enum Visibility {
    Public,
    Unlisted,
    Private,
    Direct,
}
