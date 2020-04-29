mod application;
pub(super) mod attachment;
mod card;
mod poll;

use super::account::Account;
use super::emoji::Emoji;
use super::mention::Mention;
use super::tag::Tag;
use super::visibility::Visibility;
use super::Payload;
use crate::Id;
use application::Application;
use attachment::Attachment;
use card::Card;
use hashbrown::HashSet;
use poll::Poll;
use serde::{Deserialize, Serialize};
use std::boxed::Box;
use std::string::String;

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub(super) id: Id,
    pub(super) uri: String,
    pub(super) created_at: String,
    pub(super) account: Account,
    pub(super) content: String,
    pub(super) visibility: Visibility,
    pub(super) sensitive: bool,
    pub(super) spoiler_text: String,
    pub(super) media_attachments: Vec<Attachment>,
    pub(super) application: Option<Application>, // Should be non-optional?
    pub(super) mentions: Vec<Mention>,
    pub(super) tags: Vec<Tag>,
    pub(super) emojis: Vec<Emoji>,
    pub(super) reblogs_count: i64,
    pub(super) favourites_count: i64,
    pub(super) replies_count: i64,
    pub(super) url: Option<String>,
    pub(super) in_reply_to_id: Option<Id>,
    pub(super) in_reply_to_account_id: Option<Id>,
    pub(super) reblog: Option<Box<Status>>,
    pub(super) poll: Option<Poll>,
    pub(super) card: Option<Card>,
    pub(crate) language: Option<String>,

    pub(super) text: Option<String>,
    // ↓↓↓ Only for authorized users
    pub(super) favourited: Option<bool>,
    pub(super) reblogged: Option<bool>,
    pub(super) muted: Option<bool>,
    pub(super) bookmarked: Option<bool>,
    pub(super) pinned: Option<bool>,
}

impl Payload for Status {
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
        let mut involved_users: HashSet<Id> = self.mentions.iter().map(|m| Id(m.id.0)).collect();

        // author
        involved_users.insert(Id(self.account.id.0));
        // replied-to user
        if let Some(user_id) = self.in_reply_to_account_id {
            involved_users.insert(Id(user_id.0));
        }
        // boosted user
        if let Some(boosted_status) = self.reblog.clone() {
            involved_users.insert(Id(boosted_status.account.id.0));
        }
        involved_users
    }

    fn author(&self) -> &Id {
        &self.account.id
    }

    fn sent_from(&self) -> &str {
        let sender_username = &self.account.acct;
        sender_username.split('@').nth(1).unwrap_or_default() // default occurs when sent from local instance
    }
}
