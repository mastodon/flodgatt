use super::{account::Account, status::Status};
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Conversation {
    id: String,
    accounts: Vec<Account>,
    unread: bool,
    last_status: Option<Status>,
}
