use super::{EventErr, Id};
use crate::parse_client_request::Blocks;

use std::convert::TryFrom;

use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DynEvent {
    #[serde(skip)]
    pub kind: EventKind,
    pub event: String,
    pub payload: Value,
    pub queued_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    Update(DynStatus),
    NonUpdate,
}

impl Default for EventKind {
    fn default() -> Self {
        Self::NonUpdate
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynStatus {
    pub id: Id,
    pub username: String,
    pub language: Option<String>,
    pub mentioned_users: HashSet<Id>,
    pub replied_to_user: Option<Id>,
    pub boosted_user: Option<Id>,
}

type Result<T> = std::result::Result<T, EventErr>;

impl DynEvent {
    pub fn set_update(self) -> Result<Self> {
        if self.event == "update" {
            let kind = EventKind::Update(DynStatus::new(self.payload.clone())?);
            Ok(Self { kind, ..self })
        } else {
            Ok(self)
        }
    }
}

impl DynStatus {
    pub fn new(payload: Value) -> Result<Self> {
        use EventErr::*;

        Ok(Self {
            id: Id::try_from(&payload["account"]["id"])?,
            username: payload["account"]["acct"]
                .as_str()
                .ok_or(DynParse)?
                .to_string(),
            language: payload["language"].as_str().map(|s| s.to_string()),
            mentioned_users: HashSet::new(),
            replied_to_user: Id::try_from(&payload["in_reply_to_account_id"]).ok(),
            boosted_user: Id::try_from(&payload["reblog"]["account"]["id"]).ok(),
        })
    }
    /// Returns `true` if the status is filtered out based on its language
    pub fn language_not(&self, allowed_langs: &HashSet<String>) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;

        if allowed_langs.is_empty() {
            return ALLOW; // listing no allowed_langs results in allowing all languages
        }

        match self.language.clone() {
            Some(toot_language) if allowed_langs.contains(&toot_language) => ALLOW, //
            None => ALLOW, // If toot language is unknown, toot is always allowed
            Some(empty) if empty == String::new() => ALLOW,
            Some(_toot_language) => REJECT,
        }
    }

    /// Returns `true` if the toot contained in this Event originated from a blocked domain,
    /// is from an account that has blocked the current user, or if the User's list of
    /// blocked/muted users includes a user involved in the toot.
    ///
    /// A user is involved in the toot if they:
    ///  * Are mentioned in this toot
    ///  * Wrote this toot
    ///  * Wrote a toot that this toot is replying to (if any)
    ///  * Wrote the toot that this toot is boosting (if any)
    pub fn involves_any(&self, blocks: &Blocks) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;
        let Blocks {
            blocked_users,
            blocking_users,
            blocked_domains,
        } = blocks;

        if self.involves(blocked_users) || blocking_users.contains(&self.id) {
            REJECT
        } else {
            match self.username.split('@').nth(1) {
                Some(originating_domain) if blocked_domains.contains(originating_domain) => REJECT,
                Some(_) | None => ALLOW, // None means the local instance, which can't be blocked
            }
        }
    }

    // involved_users = mentioned_users + author + replied-to user + boosted user
    fn involves(&self, blocked_users: &HashSet<Id>) -> bool {
        // mentions
        let mut involved_users: HashSet<Id> = self.mentioned_users.clone();

        // author
        involved_users.insert(self.id);

        // replied-to user
        if let Some(user_id) = self.replied_to_user {
            involved_users.insert(user_id);
        }

        // boosted user
        if let Some(user_id) = self.boosted_user {
            involved_users.insert(user_id);
        }

        !involved_users.is_disjoint(blocked_users)
    }
}
