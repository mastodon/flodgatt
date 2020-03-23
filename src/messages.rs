use crate::log_fatal;
use serde::{Deserialize, Serialize};
use serde_json;
use std::boxed::Box;
use std::{collections::HashSet, string::String};

#[serde(rename_all = "snake_case", tag = "event")]
#[rustfmt::skip]
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum Event {
    Update{ payload: Status},
    Notification{payload: Notification},
    Delete{payload: DeletedId},
    FiltersChanged,
    Announcement{payload: Announcement},
    #[serde(rename(serialize = "announcement.reaction", deserialize = "announcement.reaction"))]
    AnnouncementReaction{payload: AnnouncementReaction},
    #[serde(rename(serialize = "announcement.delete", deserialize = "announcement.delete"))]
    AnnouncementDelete{payload: DeletedId},
    Conversation{payload: Conversation},
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
            Update { payload: status } => Some(escaped(status)),
            Notification { payload: notification } => Some(escaped(notification)),
            Delete { payload: id } => Some(id.0.clone()),
            Announcement { payload: announcement } => Some(escaped(announcement)),
            AnnouncementReaction { payload: reaction } => Some(escaped(reaction)),
            AnnouncementDelete { payload: id } => Some(id.0.clone()),
            Conversation { payload: conversation} => Some(escaped(conversation)),
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Conversation {
    id: String,
    accounts: Vec<Account>,
    unread: bool,
    last_status: Option<Status>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeletedId(String);

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
    in_reply_to: Option<String>,
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

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Unlisted,
    Private,
    Dirrect,
}

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
}

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

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum AttachmentType {
    Unknown,
    Image,
    Gifv,
    Video,
    Audio,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Application {
    name: String,
    website: Option<String>,
    vapid_key: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Emoji {
    shortcode: String,
    url: String,
    static_url: String,
    visible_in_picker: bool,
    category: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Field {
    name: String,
    value: String,
    verified_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Source {
    note: String,
    fields: Vec<Field>,
    privacy: Option<Visibility>,
    sensitive: bool,
    language: String,
    follow_requests_count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Mention {
    id: String,
    username: String,
    acct: String,
    url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Tag {
    name: String,
    url: String,
    history: Option<Vec<History>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct PollOptions {
    title: String,
    votes_count: Option<i32>,
}

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

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum CardType {
    Link,
    Photo,
    Video,
    Rich,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct History {
    day: String,
    uses: String,
    accounts: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Notification {
    id: String,
    r#type: NotificationType,
    created_at: String,
    account: Account,
    status: Status,
}

#[serde(rename_all = "lowercase")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum NotificationType {
    Follow,
    Mention,
    Reblog,
    Favourite,
    Poll,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
mod test {
    use super::*;
    use crate::{parse_client_request::subscription::Timeline,
                redis_to_client_stream::{receiver::{MsgQueue, MessageQueues}, redis::{redis_stream, redis_msg}}};
    use lru::LruCache;
    use uuid::Uuid;
    use std::collections::HashMap;

    /// Set up state shared between multiple tests of Redis parsing
    pub fn shared_setup() -> (LruCache<String, i64>, MessageQueues, Uuid, Timeline) {
        let cache: LruCache<String, i64> = LruCache::new(1000);
        let mut queues_map = HashMap::new();
        let id = Uuid::default();

        let timeline = Timeline::from_redis_str("4", None);
        queues_map.insert(id, MsgQueue::new(timeline));
        let queues = MessageQueues(queues_map);
        (cache, queues, id, timeline)
    }

    const INPUT: &str  ="*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:4\r\n$1386\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102866835379605039\",\"created_at\":\"2019-09-27T22:29:02.590Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"http://localhost:3000/users/admin/statuses/102866835379605039\",\"url\":\"http://localhost:3000/@admin/102866835379605039\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"http://localhost:3000/@susan\\\" class=\\\"u-url mention\\\">@<span>susan</span></a></span> hi</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"1\",\"username\":\"admin\",\"acct\":\"admin\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-07-04T00:21:05.890Z\",\"note\":\"<p></p>\",\"url\":\"http://localhost:3000/@admin\",\"avatar\":\"http://localhost:3000/avatars/original/missing.png\",\"avatar_static\":\"http://localhost:3000/avatars/original/missing.png\",\"header\":\"http://localhost:3000/headers/original/missing.png\",\"header_static\":\"http://localhost:3000/headers/original/missing.png\",\"followers_count\":3,\"following_count\":3,\"statuses_count\":192,\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"4\",\"username\":\"susan\",\"url\":\"http://localhost:3000/@susan\",\"acct\":\"susan\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1569623342825}\r\n";
    
    #[test]
    fn parse_redis_output_into_event() {
        let input = INPUT.to_string();
        let (mut cache, mut queues, id, timeline) = shared_setup();

        redis_stream::process_messages(input.to_string(), &mut None, &mut cache, &mut queues);
        let parsed_event = queues
            .oldest_msg_in_target_queue(id, timeline)
            .unwrap();
        let test_event = Event::Update{ payload: Status {
            id: "102866835379605039".to_string(),
            created_at: "2019-09-27T22:29:02.590Z".to_string(),
            in_reply_to: None,
            in_reply_to_account_id: None,
            sensitive: false,
            spoiler_text: "".to_string(),
            visibility: Visibility::Public,
            language: Some("en".to_string()),
            uri: "http://localhost:3000/users/admin/statuses/102866835379605039".to_string(),
            url: Some("http://localhost:3000/@admin/102866835379605039".to_string()),
            replies_count: 0,
            reblogs_count: 0,
            favourites_count: 0,
            favourited: Some(false),
            reblogged: Some(false),
            muted: Some(false),
            bookmarked: None,
            pinned: None,
            content: "<p><span class=\"h-card\"><a href=\"http://localhost:3000/@susan\" class=\"u-url mention\">@<span>susan</span></a></span> hi</p>".to_string(),
            reblog: None,
            application: Some(Application {
                name: "Web".to_string(),
                website: None,
                vapid_key: None,
                client_id: None,
                client_secret: None,
            }),
            
            account: Account {
                id: "1".to_string(),
                username: "admin".to_string(),
                acct: "admin".to_string(),
                display_name: "".to_string(),
                locked:false,
                bot:Some(false),
                created_at: "2019-07-04T00:21:05.890Z".to_string(),
                note:"<p></p>".to_string(),
                url:"http://localhost:3000/@admin".to_string(),
                avatar: "http://localhost:3000/avatars/original/missing.png".to_string(),
                avatar_static:"http://localhost:3000/avatars/original/missing.png".to_string(),
                header: "http://localhost:3000/headers/original/missing.png".to_string(),
                header_static:"http://localhost:3000/headers/original/missing.png".to_string(),
                followers_count:3,
                following_count:3,
                statuses_count:192,
                emojis:vec![],
                fields:Some(vec![]),
                moved: None,
                discoverable: None,
                source: None,
                
                    
            },
            media_attachments:vec![],
            mentions: vec![ Mention {id:"4".to_string(),
                                     username:"susan".to_string(),
                                     url:"http://localhost:3000/@susan".to_string(),
                                     acct:"susan".to_string()}],
            tags:vec![],
            emojis:vec![],
            card:None,poll:None,
            text: None,
            
            
            
        }};
        dbg!(&parsed_event, &test_event);
        assert_eq!(parsed_event, test_event);
    }

    #[test]
    fn trivial_redis_parse() {
        let input = "*3\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntimeline:1\r\n:1\r\n";
        let mut msg = redis_msg::RedisMsg::from_raw(input, "timeline".len());
        let cmd = msg.next_field();
        assert_eq!(&cmd, "SUBSCRIBE");
        let timeline = msg.next_field();
        assert_eq!(&timeline, "timeline:1");
        msg.cursor += ":1\r\n".len();
        assert_eq!(msg.cursor, input.len());
    }

    #[test]
    fn realistic_redis_parse() {
        let input = "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:4\r\n$1386\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102866835379605039\",\"created_at\":\"2019-09-27T22:29:02.590Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"http://localhost:3000/users/admin/statuses/102866835379605039\",\"url\":\"http://localhost:3000/@admin/102866835379605039\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"http://localhost:3000/@susan\\\" class=\\\"u-url mention\\\">@<span>susan</span></a></span> hi</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"1\",\"username\":\"admin\",\"acct\":\"admin\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-07-04T00:21:05.890Z\",\"note\":\"<p></p>\",\"url\":\"http://localhost:3000/@admin\",\"avatar\":\"http://localhost:3000/avatars/original/missing.png\",\"avatar_static\":\"http://localhost:3000/avatars/original/missing.png\",\"header\":\"http://localhost:3000/headers/original/missing.png\",\"header_static\":\"http://localhost:3000/headers/original/missing.png\",\"followers_count\":3,\"following_count\":3,\"statuses_count\":192,\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"4\",\"username\":\"susan\",\"url\":\"http://localhost:3000/@susan\",\"acct\":\"susan\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1569623342825}\r\n";
        let mut msg = redis_msg::RedisMsg::from_raw(input, "timeline".len());
        let cmd = msg.next_field();
        assert_eq!(&cmd, "message");
        let timeline = msg.next_field();
        assert_eq!(&timeline, "timeline:4");
        let message_str = msg.next_field();
        assert_eq!(message_str, input[41..input.len() - 2]);
        assert_eq!(msg.cursor, input.len());
    }
}
