use super::Payload;
use super::{EventErr, Id};

use std::convert::TryFrom;

use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DynEvent {
    #[serde(skip)]
    pub(crate) kind: EventKind,
    pub(crate) event: String,
    pub(crate) payload: Value,
    pub(crate) queued_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EventKind {
    Update(DynStatus),
    NonUpdate,
}

impl Default for EventKind {
    fn default() -> Self {
        Self::NonUpdate
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DynStatus {
    pub(crate) id: Id,
    pub(crate) username: String,
    pub(crate) language: Option<String>,
    pub(crate) mentioned_users: HashSet<Id>,
    pub(crate) replied_to_user: Option<Id>,
    pub(crate) boosted_user: Option<Id>,
}

type Result<T> = std::result::Result<T, EventErr>;

impl DynEvent {
    pub(crate) fn set_update(self) -> Result<Self> {
        if self.event == "update" {
            let kind = EventKind::Update(DynStatus::new(&self.payload.clone())?);
            Ok(Self { kind, ..self })
        } else {
            Ok(self)
        }
    }
}
impl DynStatus {
    pub(crate) fn new(payload: &Value) -> Result<Self> {
        use EventErr::*;

        Ok(Self {
            id: Id::try_from(&payload["account"]["id"])?,
            username: payload["account"]["acct"]
                .as_str()
                .ok_or(DynParse)?
                .to_string(),
            language: payload["language"].as_str().map(String::from),
            mentioned_users: HashSet::new(),
            replied_to_user: Id::try_from(&payload["in_reply_to_account_id"]).ok(),
            boosted_user: Id::try_from(&payload["reblog"]["account"]["id"]).ok(),
        })
    }
}

impl Payload for DynStatus {
    fn language_unset(&self) -> bool {
        match &self.language {
            None => true,
            Some(empty) if empty == &String::new() => true,
            Some(_language) => false,
        }
    }

    fn language(&self) -> String {
        self.language.clone().unwrap_or_default()
    }
    /// Returns all users involved in the `Status`.
    ///
    /// A user is involved in the Status/toot if they:
    ///  * Are mentioned in this toot
    ///  * Wrote this toot
    ///  * Wrote a toot that this toot is replying to (if any)
    ///  * Wrote the toot that this toot is boosting (if any)
    fn involved_users(&self) -> HashSet<Id> {
        // involved_users = mentioned_users + author + replied-to user + boosted user
        let mut involved_users: HashSet<Id> = self.mentioned_users.clone();

        // author
        involved_users.insert(self.id);
        // replied-to user
        if let Some(user_id) = self.replied_to_user {
            involved_users.insert(user_id);
        }
        // boosted user
        if let Some(boosted_status) = self.boosted_user {
            involved_users.insert(boosted_status);
        }
        involved_users
    }

    fn author(&self) -> &Id {
        &self.id
    }

    fn sent_from(&self) -> &str {
        let sender_username = &self.username;
        sender_username.split('@').nth(1).unwrap_or_default() // default occurs when sent from local instance
    }
}
