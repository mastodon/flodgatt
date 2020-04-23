use super::{emoji::Emoji, mention::Mention, tag::Tag, AnnouncementReaction};
use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
