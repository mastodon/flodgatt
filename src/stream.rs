//! Manage all existing Redis PubSub connection
use crate::receiver::Receiver;
use crate::user::{Filter, User};
use futures::stream::Stream;
use futures::{Async, Poll};
use serde_json::json;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::io::Error;
use uuid::Uuid;

/// Struct for manageing all Redis streams
#[derive(Clone, Debug)]
pub struct StreamManager {
    receiver: Arc<Mutex<Receiver>>,
    id: uuid::Uuid,
    target_timeline: String,
    current_user: Option<User>,
}
impl StreamManager {
    pub fn new(reciever: Receiver) -> Self {
        StreamManager {
            receiver: Arc::new(Mutex::new(reciever)),
            id: Uuid::default(),
            target_timeline: String::new(),
            current_user: None,
        }
    }

    /// Create a blank StreamManager copy
    pub fn blank_copy(&self) -> Self {
        StreamManager { ..self.clone() }
    }
    /// Create a StreamManager copy with a new unique id manage subscriptions
    pub fn configure_copy(&self, timeline: &String, user: User) -> Self {
        let id = Uuid::new_v4();
        let mut receiver = self.receiver.lock().expect("No panic in other threads");
        receiver.update(id, timeline);
        receiver.maybe_subscribe(timeline);
        StreamManager {
            id,
            current_user: Some(user),
            target_timeline: timeline.clone(),
            ..self.clone()
        }
    }
}

impl Stream for StreamManager {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut receiver = self
            .receiver
            .lock()
            .expect("StreamManager: No other thread panic");
        receiver.update(self.id, &self.target_timeline.clone());
        match receiver.poll() {
            Ok(Async::Ready(Some(value))) => {
                let user = self
                    .clone()
                    .current_user
                    .expect("Previously set current user");

                let user_langs = user.langs.clone();
                let event = value["event"].as_str().expect("Redis string");
                let payload = value["payload"].to_string();

                match (&user.filter, user_langs) {
                    (Filter::Notification, _) if event != "notification" => Ok(Async::NotReady),
                    (Filter::Language, Some(ref user_langs))
                        if !user_langs.contains(
                            &value["payload"]["language"]
                                .as_str()
                                .expect("Redis str")
                                .to_string(),
                        ) =>
                    {
                        Ok(Async::NotReady)
                    }
                    _ => Ok(Async::Ready(Some(json!(
                        {"event": event,
                         "payload": payload,}
                    )))),
                }
            }
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
