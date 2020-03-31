use crate::log_fatal;
use serde::{Deserialize, Serialize};
use serde_json;
use std::boxed::Box;
use std::{collections::HashSet, string::String};

#[serde(rename_all = "snake_case", tag = "event", deny_unknown_fields)]
#[rustfmt::skip]
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum Event {
    Update { payload: Status, queued_at: Option<i64> },
    Notification { payload: Notification },
    Delete { payload: DeletedId },
    FiltersChanged,
    Announcement { payload: Announcement },
    #[serde(rename(serialize = "announcement.reaction", deserialize = "announcement.reaction"))]
    AnnouncementReaction { payload: AnnouncementReaction },
    #[serde(rename(serialize = "announcement.delete", deserialize = "announcement.delete"))]
    AnnouncementDelete { payload: DeletedId },
    Conversation { payload: Conversation, queued_at: Option<i64> },
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum SendableEvent<'a> {
    WithPayload { event: &'a str, payload: String },
    NoPayload { event: &'a str },
}
#[rustfmt::skip]
impl Event {
    pub fn event_name(&self) -> String {
        use Event::*;
        match self {
            Update { .. } => "update",
            Notification { .. } => "notification",
            Delete { .. } => "delete",
            Announcement { .. } => "announcement",
            AnnouncementReaction { .. } => "announcement.reaction",
            AnnouncementDelete { .. } => "announcement.delete",
            Conversation { .. } => "conversation",
            FiltersChanged => "filters_changed",
        }
        .to_string()
    }

            
    pub fn payload(&self) -> Option<String> {
        use Event::*;
        match self {
            Update { payload: status, .. } => Some(escaped(status)),
            Notification { payload: notification, .. } => Some(escaped(notification)),
            Delete { payload: id, .. } => Some(id.0.clone()),
            Announcement { payload: announcement, .. } => Some(escaped(announcement)),
            AnnouncementReaction { payload: reaction, .. } => Some(escaped(reaction)),
            AnnouncementDelete { payload: id, .. } => Some(id.0.clone()),
            Conversation { payload: conversation, ..} => Some(escaped(conversation)),
            FiltersChanged => None,
        }
    }
    pub fn to_json_string(&self) -> String {
        let event = &self.event_name();
        let sendable_event = match self.payload() {
            Some(payload) => SendableEvent::WithPayload { event, payload },
            None => SendableEvent::NoPayload { event },
        };
        serde_json::to_string(&sendable_event)
            .unwrap_or_else(|_| log_fatal!("Could not serialize `{:?}`", &sendable_event))
    }
}

fn escaped<T: Serialize + std::fmt::Debug>(content: T) -> String {
    serde_json::to_string(&content)
        .unwrap_or_else(|_| log_fatal!("Could not parse Event with: `{:?}`", &content))
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Conversation {
    id: String,
    accounts: Vec<Account>,
    unread: bool,
    last_status: Option<Status>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeletedId(String);

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

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Unlisted,
    Private,
    Direct,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Account {
    id: String,
    username: String,
    acct: String,
    url: String,
    display_name: String,
    note: String,
    avatar: String,
    avatar_static: String,
    header: String,
    header_static: String,
    locked: bool,
    emojis: Vec<Emoji>,
    discoverable: Option<bool>, // Shouldn't be option?
    created_at: String,
    statuses_count: i64,
    followers_count: i64,
    following_count: i64,
    moved: Option<Box<String>>,
    fields: Option<Vec<Field>>,
    bot: Option<bool>,
    source: Option<Source>,
    group: Option<bool>,            // undocumented
    last_status_at: Option<String>, // undocumented
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Attachment {
    id: String,
    r#type: AttachmentType,
    url: String,
    preview_url: String,
    remote_url: Option<String>,
    text_url: Option<String>,
    meta: Option<serde_json::Value>,
    description: Option<String>,
    blurhash: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum AttachmentType {
    Unknown,
    Image,
    Gifv,
    Video,
    Audio,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Application {
    name: String,
    website: Option<String>,
    vapid_key: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Emoji {
    shortcode: String,
    url: String,
    static_url: String,
    visible_in_picker: bool,
    category: Option<String>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Field {
    name: String,
    value: String,
    verified_at: Option<String>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Source {
    note: String,
    fields: Vec<Field>,
    privacy: Option<Visibility>,
    sensitive: bool,
    language: String,
    follow_requests_count: i64,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Mention {
    id: String,
    username: String,
    acct: String,
    url: String,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Tag {
    name: String,
    url: String,
    history: Option<Vec<History>>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Poll {
    id: String,
    expires_at: String,
    expired: bool,
    multiple: bool,
    votes_count: i64,
    voters_count: Option<i64>,
    voted: Option<bool>,
    own_votes: Option<Vec<i64>>,
    options: Vec<PollOptions>,
    emojis: Vec<Emoji>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct PollOptions {
    title: String,
    votes_count: Option<i32>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Card {
    url: String,
    title: String,
    description: String,
    r#type: CardType,
    author_name: Option<String>,
    author_url: Option<String>,
    provider_name: Option<String>,
    provider_url: Option<String>,
    html: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    image: Option<String>,
    embed_url: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum CardType {
    Link,
    Photo,
    Video,
    Rich,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct History {
    day: String,
    uses: String,
    accounts: String,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Notification {
    id: String,
    r#type: NotificationType,
    created_at: String,
    account: Account,
    status: Option<Status>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum NotificationType {
    Follow,
    Mention,
    Reblog,
    Favourite,
    Poll,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Announcement {
    // Fully undocumented
    id: String,
    tags: Vec<Tag>,
    all_day: bool,
    content: String,
    emojis: Vec<Emoji>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    published_at: String,
    updated_at: String,
    mentions: Vec<Mention>,
    reactions: Vec<AnnouncementReaction>,
}

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AnnouncementReaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    announcement_id: Option<String>,
    count: i64,
    name: String,
}

impl Status {
    /// Returns `true` if the status is filtered out based on its language
    pub fn language_not_allowed(&self, allowed_langs: &HashSet<String>) -> bool {
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

    /// Returns `true` if this toot originated from a domain the User has blocked.
    pub fn from_blocked_domain(&self, blocked_domains: &HashSet<String>) -> bool {
        let full_username = &self.account.acct;

        match full_username.split('@').nth(1) {
            Some(originating_domain) => blocked_domains.contains(originating_domain),
            None => false, // None means the user is on the local instance, which can't be blocked
        }
    }
    /// Returns `true` if the Status is from an account that has blocked the current user.
    pub fn from_blocking_user(&self, blocking_users: &HashSet<i64>) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;
        let err = |_| log_fatal!("Could not process `account.id` in {:?}", &self);

        if blocking_users.contains(&self.account.id.parse().unwrap_or_else(err)) {
            REJECT
        } else {
            ALLOW
        }
    }

    /// Returns `true` if the User's list of blocked and muted users includes a user
    /// involved in this toot.
    ///
    /// A user is involved if they:
    ///  * Are mentioned in this toot
    ///  * Wrote this toot
    ///  * Wrote a toot that this toot is replying to (if any)
    ///  * Wrote the toot that this toot is boosting (if any)
    pub fn involves_blocked_user(&self, blocked_users: &HashSet<i64>) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;
        let err = |_| log_fatal!("Could not process an `id` field in {:?}", &self);

        // involved_users = mentioned_users + author + replied-to user + boosted user
        let mut involved_users: HashSet<i64> = self
            .mentions
            .iter()
            .map(|mention| mention.id.parse().unwrap_or_else(err))
            .collect();

        involved_users.insert(self.account.id.parse::<i64>().unwrap_or_else(err));

        if let Some(replied_to_account_id) = self.in_reply_to_account_id.clone() {
            involved_users.insert(replied_to_account_id.parse().unwrap_or_else(err));
        }

        if let Some(boosted_status) = self.reblog.clone() {
            involved_users.insert(boosted_status.account.id.parse().unwrap_or_else(err));
        }

        if involved_users.is_disjoint(blocked_users) {
            ALLOW
        } else {
            REJECT
        }
    }
}

#[cfg(test)]
mod test;
