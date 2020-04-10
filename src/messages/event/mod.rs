mod checked_event;
mod dynamic_event;

pub use {checked_event::CheckedEvent, dynamic_event::DynamicEvent};

use crate::log_fatal;
use crate::redis_to_client_stream::ReceiverErr;
use serde::Serialize;
use std::{convert::TryFrom, string::String};

#[derive(Debug, Clone)]
pub enum Event {
    TypeSafe(CheckedEvent),
    Dynamic(DynamicEvent),
    Ping,
}

impl Event {
    pub fn to_json_string(&self) -> String {
        let event = &self.event_name();
        let sendable_event = match self.payload() {
            Some(payload) => SendableEvent::WithPayload { event, payload },
            None => SendableEvent::NoPayload { event },
        };
        serde_json::to_string(&sendable_event)
            .unwrap_or_else(|_| log_fatal!("Could not serialize `{:?}`", &sendable_event))
    }

    pub fn event_name(&self) -> String {
        String::from(match self {
            Self::TypeSafe(checked) => match checked {
                CheckedEvent::Update { .. } => "update",
                CheckedEvent::Notification { .. } => "notification",
                CheckedEvent::Delete { .. } => "delete",
                CheckedEvent::Announcement { .. } => "announcement",
                CheckedEvent::AnnouncementReaction { .. } => "announcement.reaction",
                CheckedEvent::AnnouncementDelete { .. } => "announcement.delete",
                CheckedEvent::Conversation { .. } => "conversation",
                CheckedEvent::FiltersChanged => "filters_changed",
            },
            Self::Dynamic(dyn_event) => &dyn_event.event,
            Self::Ping => panic!("event_name() called on EventNotReady"),
        })
    }

    pub fn payload(&self) -> Option<String> {
        use CheckedEvent::*;
        match self {
            Self::TypeSafe(checked) => match checked {
                Update { payload, .. } => Some(escaped(payload)),
                Notification { payload, .. } => Some(escaped(payload)),
                Delete { payload, .. } => Some(payload.clone()),
                Announcement { payload, .. } => Some(escaped(payload)),
                AnnouncementReaction { payload, .. } => Some(escaped(payload)),
                AnnouncementDelete { payload, .. } => Some(payload.clone()),
                Conversation { payload, .. } => Some(escaped(payload)),
                FiltersChanged => None,
            },
            Self::Dynamic(dyn_event) => Some(dyn_event.payload.to_string()),
            Self::Ping => panic!("payload() called on EventNotReady"),
        }
    }
}

impl TryFrom<String> for Event {
    type Error = ReceiverErr;
    fn try_from(event_txt: String) -> Result<Event, ReceiverErr> {
        Event::try_from(event_txt.as_str())
    }
}
impl TryFrom<&str> for Event {
    type Error = ReceiverErr;
    fn try_from(event_txt: &str) -> Result<Event, ReceiverErr> {
        match serde_json::from_str(event_txt) {
            Ok(checked_event) => Ok(Event::TypeSafe(checked_event)),
            Err(e) => {
                log::error!(
                    "Error safely parsing Redis input.  Mastodon and Flodgatt do not \
                             strictly conform to the same version of Mastodon's API.\n{}\n\
                             Forwarding Redis payload without type checking it.",
                    e
                );
                let dyn_event: DynamicEvent = serde_json::from_str(&event_txt)?;
                Ok(Event::Dynamic(dyn_event))
            }
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
enum SendableEvent<'a> {
    WithPayload { event: &'a str, payload: String },
    NoPayload { event: &'a str },
}

fn escaped<T: Serialize + std::fmt::Debug>(content: T) -> String {
    serde_json::to_string(&content)
        .unwrap_or_else(|_| log_fatal!("Could not parse Event with: `{:?}`", &content))
}
