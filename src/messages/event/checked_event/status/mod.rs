mod application;
mod attachment;
mod card;
mod poll;

use super::{account::Account, emoji::Emoji, mention::Mention, tag::Tag, visibility::Visibility};
use {application::Application, attachment::Attachment, card::Card, poll::Poll};

use crate::log_fatal;
use crate::parse_client_request::Blocks;

use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use std::boxed::Box;
use std::string::String;

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Status {
    id: String,
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
    in_reply_to_id: Option<String>,
    in_reply_to_account_id: Option<String>,
    reblog: Option<Box<Status>>,
    poll: Option<Poll>,
    card: Option<Card>,
    language: Option<String>,

    text: Option<String>,
    // ↓↓↓ Only for authorized users
    favourited: Option<bool>,
    reblogged: Option<bool>,
    muted: Option<bool>,
    bookmarked: Option<bool>,
    pinned: Option<bool>,
}

impl Status {
    /// Returns `true` if the status is filtered out based on its language
    pub fn language_not(&self, allowed_langs: &HashSet<String>) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;

        let reject_and_maybe_log = |toot_language| {
            log::info!("Filtering out toot from `{}`", &self.account.acct);
            log::info!("Toot language: `{}`", toot_language);
            log::info!("Recipient's allowed languages: `{:?}`", allowed_langs);
            REJECT
        };
        if allowed_langs.is_empty() {
            return ALLOW; // listing no allowed_langs results in allowing all languages
        }

        match self.language.as_ref() {
            Some(toot_language) if allowed_langs.contains(toot_language) => ALLOW,
            None => ALLOW, // If toot language is unknown, toot is always allowed
            Some(empty) if empty == &String::new() => ALLOW,
            Some(toot_language) => reject_and_maybe_log(toot_language),
        }
    }

    /// Returns `true` if the Status originated from a blocked domain, is from an account
    /// that has blocked the current user, or if the User's list of blocked/muted users
    /// includes a user involved in the Status.
    ///
    /// A user is involved in the Status/toot if they:
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
        let user_id = &self.account.id.parse().expect("TODO");

        if blocking_users.contains(user_id) || self.involves(blocked_users) {
            REJECT
        } else {
            let full_username = &self.account.acct;
            match full_username.split('@').nth(1) {
                Some(originating_domain) if blocked_domains.contains(originating_domain) => REJECT,
                Some(_) | None => ALLOW, // None means the local instance, which can't be blocked
            }
        }
    }

    fn involves(&self, blocked_users: &HashSet<i64>) -> bool {
        // TODO replace vvvv with error handling
        let err = |_| log_fatal!("Could not process an `id` field in {:?}", &self);

        // involved_users = mentioned_users + author + replied-to user + boosted user
        let mut involved_users: HashSet<i64> = self
            .mentions
            .iter()
            .map(|mention| mention.id.parse().unwrap_or_else(err))
            .collect();

        // author
        involved_users.insert(self.account.id.parse::<i64>().unwrap_or_else(err));
        // replied-to user
        if let Some(user_id) = self.in_reply_to_account_id.clone() {
            involved_users.insert(user_id.parse().unwrap_or_else(err));
        }
        // boosted user
        if let Some(boosted_status) = self.reblog.clone() {
            involved_users.insert(boosted_status.account.id.parse().unwrap_or_else(err));
        }
        !involved_users.is_disjoint(blocked_users)
    }
}
