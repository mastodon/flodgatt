use super::{account::Account, status::Status};
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    id: String,
    r#type: NotificationType,
    created_at: String,
    account: Account,
    status: Option<Status>,
}

#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum NotificationType {
    Follow,
    FollowRequest, // Undocumented
    Mention,
    Reblog,
    Favourite,
    Poll,
}
