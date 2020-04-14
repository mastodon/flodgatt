mod checked_event;
mod dynamic_event;
mod err;

pub use checked_event::{CheckedEvent, Id};
pub use dynamic_event::{DynEvent, DynStatus, EventKind};
pub use err::EventErr;

use serde::Serialize;
use std::convert::TryFrom;
use std::string::String;
use warp::sse::ServerSentEvent;

#[derive(Debug, Clone)]
pub enum Event {
    TypeSafe(CheckedEvent),
    Dynamic(DynEvent),
    Ping,
}

impl Event {
    pub fn to_json_string(&self) -> String {
        if let Event::Ping = self {
            "{}".to_string()
        } else {
            let event = &self.event_name();
            let sendable_event = match self.payload() {
                Some(payload) => SendableEvent::WithPayload { event, payload },
                None => SendableEvent::NoPayload { event },
            };
            serde_json::to_string(&sendable_event).expect("Guaranteed: SendableEvent is Serialize")
        }
    }

    pub fn to_warp_reply(&self) -> Option<(impl ServerSentEvent, impl ServerSentEvent)> {
        if let Event::Ping = self {
            None
        } else {
            Some((
                warp::sse::event(self.event_name()),
                warp::sse::data(self.payload().unwrap_or_else(String::new)),
            ))
        }
    }

    fn event_name(&self) -> String {
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
            Self::Dynamic(DynEvent {
                kind: EventKind::Update(_),
                ..
            }) => "update",
            Self::Dynamic(DynEvent { event, .. }) => event,
            Self::Ping => unreachable!(), // private method only called above
        })
    }

    fn payload(&self) -> Option<String> {
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
            Self::Dynamic(DynEvent { payload, .. }) => Some(payload.to_string()),
            Self::Ping => unreachable!(), // private method only called above
        }
    }
}

impl TryFrom<String> for Event {
    type Error = EventErr;

    fn try_from(event_txt: String) -> Result<Event, Self::Error> {
        Event::try_from(event_txt.as_str())
    }
}
impl TryFrom<&str> for Event {
    type Error = EventErr;

    fn try_from(event_txt: &str) -> Result<Event, Self::Error> {
        match serde_json::from_str(event_txt) {
            Ok(checked_event) => Ok(Event::TypeSafe(checked_event)),
            Err(e) => {
                log::error!(
                    "Error safely parsing Redis input.  Mastodon and Flodgatt do not \
                             strictly conform to the same version of Mastodon's API.\n{}\n\
                             Forwarding Redis payload without type checking it.",
                    e
                );

                Ok(Event::Dynamic(serde_json::from_str(&event_txt)?))
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
    serde_json::to_string(&content).expect("Guaranteed by Serialize trait bound")
}
