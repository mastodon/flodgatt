use crate::log_fatal;
use crate::messages::Event;
use serde_json::Value;
use std::{collections::HashSet, string::String};
use strum_macros::Display;

#[derive(Debug, Display, Clone)]
pub enum Message {
    Update(Status),
    Conversation(Value),
    Notification(Value),
    Delete(String),
    FiltersChanged,
    Announcement(AnnouncementType),
    UnknownEvent(String, Value),
}

#[derive(Debug, Clone)]
pub struct Status(Value);

#[derive(Debug, Clone)]
pub enum AnnouncementType {
    New(Value),
    Delete(String),
    Reaction(Value),
}

impl Message {
    // pub fn from_json(event: Event) -> Self {
    //     use AnnouncementType::*;

    //     match event.event.as_ref() {
    //         "update" => Self::Update(Status(event.payload)),
    //         "conversation" => Self::Conversation(event.payload),
    //         "notification" => Self::Notification(event.payload),
    //         "delete" => Self::Delete(
    //             event
    //                 .payload
    //                 .as_str()
    //                 .unwrap_or_else(|| log_fatal!("Could not process `payload` in {:?}", event))
    //                 .to_string(),
    //         ),
    //         "filters_changed" => Self::FiltersChanged,
    //         "announcement" => Self::Announcement(New(event.payload)),
    //         "announcement.reaction" => Self::Announcement(Reaction(event.payload)),
    //         "announcement.delete" => Self::Announcement(Delete(
    //             event
    //                 .payload
    //                 .as_str()
    //                 .unwrap_or_else(|| log_fatal!("Could not process `payload` in {:?}", event))
    //                 .to_string(),
    //         )),
    //         other => {
    //             log::warn!("Received unexpected `event` from Redis: {}", other);
    //             Self::UnknownEvent(event.event.to_string(), event.payload)
    //         }
    //     }
    // }
    pub fn event(&self) -> String {
        use AnnouncementType::*;
        match self {
            Self::Update(_) => "update",
            Self::Conversation(_) => "conversation",
            Self::Notification(_) => "notification",
            Self::Announcement(New(_)) => "announcement",
            Self::Announcement(Reaction(_)) => "announcement.reaction",
            Self::UnknownEvent(event, _) => &event,
            Self::Delete(_) => "delete",
            Self::Announcement(Delete(_)) => "announcement.delete",
            Self::FiltersChanged => "filters_changed",
        }
        .to_string()
    }
    pub fn payload(&self) -> String {
        use AnnouncementType::*;
        match self {
            Self::Update(status) => status.0.to_string(),
            Self::Conversation(value)
            | Self::Notification(value)
            | Self::Announcement(New(value))
            | Self::Announcement(Reaction(value))
            | Self::UnknownEvent(_, value) => value.to_string(),
            Self::Delete(id) | Self::Announcement(Delete(id)) => id.clone(),
            Self::FiltersChanged => "".to_string(),
        }
    }
}
