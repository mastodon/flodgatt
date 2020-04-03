use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AnnouncementReaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    announcement_id: Option<String>,
    count: i64,
    name: String,
}
