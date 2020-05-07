mod application;
pub(crate) mod attachment;
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
    pub(crate) id: Id,
    pub(crate) uri: String,
    pub(crate) created_at: String,
    pub(crate) account: Account,
    pub(crate) content: String,
    pub(crate) visibility: Visibility,
    pub(crate) sensitive: bool,
    pub(crate) spoiler_text: String,
    pub(crate) media_attachments: Vec<Attachment>,
    pub(crate) application: Option<Application>, // Should be non-optional?
    pub(crate) mentions: Vec<Mention>,
    pub(crate) tags: Vec<Tag>,
    pub(crate) emojis: Vec<Emoji>,
    pub(crate) reblogs_count: i64,
    pub(crate) favourites_count: i64,
    pub(crate) replies_count: i64,
    pub(crate) url: Option<String>,
    pub(crate) in_reply_to_id: Option<Id>,
    pub(crate) in_reply_to_account_id: Option<Id>,
    pub(crate) reblog: Option<Box<Status>>,
    pub(crate) poll: Option<Poll>,
    pub(crate) card: Option<Card>,
    pub(crate) language: Option<String>,
    pub(crate) text: Option<String>,
    // ↓↓↓ Only for authorized users
    pub(crate) favourited: Option<bool>,
    pub(crate) reblogged: Option<bool>,
    pub(crate) muted: Option<bool>,
    pub(crate) bookmarked: Option<bool>,
    pub(crate) pinned: Option<bool>,
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
