mod account;

mod announcement;
mod announcement_reaction;
mod conversation;
mod emoji;
mod mention;
mod notification;
mod status;
mod tag;
mod visibility;

pub use announcement::Announcement;
pub(in crate::messages::event) use announcement_reaction::AnnouncementReaction;
pub use conversation::Conversation;
pub use notification::Notification;
pub use status::Status;

use serde::Deserialize;

#[serde(rename_all = "snake_case", tag = "event", deny_unknown_fields)]
#[rustfmt::skip]
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum CheckedEvent {
    Update { payload: Status, queued_at: Option<i64> },
    Notification { payload: Notification },
    Delete { payload: String },
    FiltersChanged,
    Announcement { payload: Announcement },
    #[serde(rename(serialize = "announcement.reaction", deserialize = "announcement.reaction"))]
    AnnouncementReaction { payload: AnnouncementReaction },
    #[serde(rename(serialize = "announcement.delete", deserialize = "announcement.delete"))]
    AnnouncementDelete { payload: String },
    Conversation { payload: Conversation, queued_at: Option<i64> },
}
