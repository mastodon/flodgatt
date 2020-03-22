use crate::log_fatal;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use std::boxed::Box;
use std::{collections::HashSet, string::String};

#[serde(rename_all = "snake_case", tag = "event")]
#[rustfmt::skip]
#[derive(Deserialize, Debug, Clone)]
pub enum Event {
    Update(Status),
    Notification(Notification),
    Delete(DeletedId),
    FiltersChanged,
    Announcement(Announcement),
    #[serde(rename(serialize = "announcement.reaction", deserialize = "announcement.reaction"))]
    AnnouncementReaction(AnnouncementReaction),
    #[serde(rename(serialize = "announcement.delete", deserialize = "announcement.delete"))]
    AnnouncementDelete(DeletedId),
    Conversation(Conversation),
}
#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum SendableEvent<'a> {
    WithPayload { event: &'a str, payload: String },
    NoPayload { event: &'a str },
}
impl Event {
    pub fn event_name(&self) -> String {
        use Event::*;
        match self {
            Update(_) => "update",
            Notification(_) => "notification",
            Delete(_) => "delete",
            Announcement(_) => "announcement",
            AnnouncementReaction(_) => "announcement.reaction",
            AnnouncementDelete(_) => "announcement.delete",
            Conversation(_) => "conversation",
            FiltersChanged => "filters_changed",
        }
        .to_string()
    }
    pub fn payload(&self) -> Option<String> {
        use Event::*;
        match self {
            Update(status) => Some(escaped(status)),
            Notification(notification) => Some(escaped(notification)),
            Delete(id) => Some(id.0.clone()),
            Announcement(announcement) => Some(escaped(announcement)),
            AnnouncementReaction(reaction) => Some(escaped(reaction)),
            AnnouncementDelete(id) => Some(id.0.clone()),
            Conversation(conversation) => Some(escaped(conversation)),
            FiltersChanged => None,
        }
    }
    pub fn to_sendable_event(&self) -> SendableEvent {
        use Event::*;
        let (event, payload) = match self {
            Update(status) => ("update", escaped(status)),
            Notification(notification) => ("notification", escaped(notification)),
            Delete(id) => ("delete", id.0.clone()),
            Announcement(announcement) => ("announcement", escaped(announcement)),
            AnnouncementReaction(reaction) => ("announcement.reaction", escaped(reaction)),
            AnnouncementDelete(id) => ("announcement.delete", id.0.clone()),
            Conversation(conversation) => ("conversation", escaped(conversation)),
            FiltersChanged => {
                return SendableEvent::NoPayload {
                    event: "filters_changed",
                }
            }
        };
        SendableEvent::WithPayload { event, payload }
    }
    pub fn to_json_string(&self) -> String {
        let event = &self.event_name();
        let sendable_event = match self.payload() {
            Some(payload) => SendableEvent::WithPayload { event, payload },
            None => SendableEvent::NoPayload { event },
        };
        serde_json::to_string(&sendable_event).expect("TODO")
    }
}

fn escaped<T: Serialize + std::fmt::Debug>(content: T) -> String {
    serde_json::to_string(&content)
        .unwrap_or_else(|_| log_fatal!("Could not parse Event with: `{:?}`", &content))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Conversation {
    id: String,
    accounts: Vec<Account>,
    unread: bool,
    last_status: Option<Status>,
}
impl ToSendable for Conversation {
    fn with_escaped_payload(&self) -> serde_json::Value {
        json!({"event": "conversation", "payload": escaped(self)})
    }
    fn with_payload(&self) -> serde_json::Value {
        json!({"event": "conversation", "payload": self})
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeletedId(String);
impl ToSendable for DeletedId {
    fn with_escaped_payload(&self) -> serde_json::Value {
        json!({"event": "delete", "payload": escaped(self)})
    }
    fn with_payload(&self) -> serde_json::Value {
        json!({"event": "delete", "payload": self})
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    in_reply_to: Option<String>,
    in_reply_to_account_id: Option<String>,
    reblog: Option<Box<Status>>,
    poll: Option<Poll>,
    card: Option<Card>,
    language: Option<String>,
    text: Option<String>,
    // plus others for Auth. users
}

impl ToSendable for Status {
    fn with_escaped_payload(&self) -> serde_json::Value {
        json!({"event": "update", "payload": escaped(self)})
    }
    fn with_payload(&self) -> serde_json::Value {
        json!({"event": "update", "payload": self})
    }
}

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum Visibility {
    Public,
    Unlisted,
    Private,
    Dirrect,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum AttachmentType {
    Unknown,
    Image,
    Gifv,
    Video,
    Audio,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Application {
    name: String,
    website: Option<String>,
    vapid_key: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Emoji {
    shortcode: String,
    url: String,
    static_url: String,
    visible_in_picker: bool,
    category: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Field {
    name: String,
    value: String,
    verified_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Source {
    note: String,
    fields: Vec<Field>,
    privacy: Option<Visibility>,
    sensitive: bool,
    language: String,
    follow_requests_count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Mention {
    id: String,
    username: String,
    acct: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Tag {
    name: String,
    url: String,
    history: Option<Vec<History>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Poll {
    id: String,
    expires_at: String,
    expired: bool,
    multiple: bool,
    votes_count: i64,
    voters_count: Option<i64>,
    voted: Option<bool>,
    own_votes: Option<i64>,
    options: Vec<PollOptions>,
    emojis: Vec<Emoji>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PollOptions {
    title: String,
    votes_count: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum CardType {
    Link,
    Photo,
    Video,
    Rich,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct History {
    day: String,
    uses: String,
    accounts: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Notification {
    id: String,
    r#type: NotificationType,
    created_at: String,
    account: Account,
    status: Status,
}
impl ToSendable for Notification {
    fn with_escaped_payload(&self) -> serde_json::Value {
        json!({"event": "notification", "payload": escaped(self)})
    }
    fn with_payload(&self) -> serde_json::Value {
        json!({"event": "notification", "payload": self})
    }
}
#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum NotificationType {
    Follow,
    Mention,
    Reblog,
    Favourite,
    Poll,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Announcement {
    id: String,
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
impl ToSendable for Announcement {
    fn with_escaped_payload(&self) -> serde_json::Value {
        json!({"event": "announcement", "payload": escaped(self)})
    }
    fn with_payload(&self) -> serde_json::Value {
        json!({"event": "announcement", "payload": self})
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnnouncementReaction {
    announcement_id: String,
    count: i64,
    name: String,
}
impl ToSendable for AnnouncementReaction {
    fn with_escaped_payload(&self) -> serde_json::Value {
        json!({"event": "announcement.reaction", "payload": escaped(self)})
    }
    fn with_payload(&self) -> serde_json::Value {
        json!({"event": "announcement.reaction", "payload": self})
    }
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
