mod application;
mod attachment;
mod card;
mod poll;

use super::account::Account;
use super::emoji::Emoji;
use super::id::Id;
use super::mention::Mention;
use super::tag::Tag;
use super::visibility::Visibility;
use super::Payload;
use application::Application;
use attachment::Attachment;
use card::Card;
use hashbrown::HashSet;
use poll::Poll;
use serde::{Deserialize, Serialize};
use std::boxed::Box;
use std::string::String;

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Status {
    id: Id,
    uri: String,
    created_at: String,
    account: Account,
    content: String,
    visibility: Visibility,
    sensitive: bool,
    spoiler_text: String,
    media_attachments: Vec<Attachment>,
    application: Option<Application>, // Should be non-optional?
    mentions: Vec<Mention>,
    tags: Vec<Tag>,
    emojis: Vec<Emoji>,
    reblogs_count: i64,
    favourites_count: i64,
    replies_count: i64,
    url: Option<String>,
    in_reply_to_id: Option<Id>,
    in_reply_to_account_id: Option<Id>,
    reblog: Option<Box<Status>>,
    poll: Option<Poll>,
    card: Option<Card>,
    pub(crate) language: Option<String>,

    text: Option<String>,
    // ↓↓↓ Only for authorized users
    favourited: Option<bool>,
    reblogged: Option<bool>,
    muted: Option<bool>,
    bookmarked: Option<bool>,
    pinned: Option<bool>,
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
