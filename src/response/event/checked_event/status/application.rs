use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Application {
    name: String,
    website: Option<String>,
    vapid_key: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}
