use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Application {
    pub(super) name: String,
    pub(super) website: Option<String>,
    pub(super) vapid_key: Option<String>,
    pub(super) client_id: Option<String>,
    pub(super) client_secret: Option<String>,
}
