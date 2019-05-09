//! Manage all existing Redis PubSub connection
use crate::receiver::Receiver;
use crate::user::User;
use futures::stream::Stream;
use futures::{Async, Poll};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::io::Error;
use uuid::Uuid;

/// Struct for manageing all Redis streams
#[derive(Clone, Debug)]
pub struct StreamManager {
    receiver: Arc<Mutex<Receiver>>,
    id: uuid::Uuid,
    current_user: Option<User>,
}
impl StreamManager {
    pub fn new(reciever: Receiver) -> Self {
        StreamManager {
            receiver: Arc::new(Mutex::new(reciever)),
            id: Uuid::new_v4(),
            current_user: None,
        }
    }

    /// Clone the StreamManager with a new unique id
    pub fn new_copy(&self) -> Self {
        let id = Uuid::new_v4();
        StreamManager { id, ..self.clone() }
    }

    /// Subscribe to a channel if not already subscribed
    ///
    ///
    /// `.add()` also unsubscribes from any channels that no longer have clients
    pub fn add(&mut self, timeline: &str, _user: &User) {
        let mut receiver = self.receiver.lock().expect("No panic in other threads");
        receiver.set_manager_id(self.id);
        receiver.subscribe(timeline);
    }

    pub fn set_user(&mut self, user: User) {
        self.current_user = Some(user);
    }
}
use crate::user::Filter;
use serde_json::json;

impl Stream for StreamManager {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut receiver = self.receiver.lock().expect("No other thread panic");
        receiver.set_manager_id(self.id);
        match receiver.poll() {
            Ok(Async::Ready(Some(value))) => {
                let user = self
                    .clone()
                    .current_user
                    .expect("Previously set current user");

                let user_langs = user.langs.clone();
                let copy = value.clone();
                let event = copy["event"].as_str().expect("Redis string");
                let copy = value.clone();
                let payload = copy["payload"].to_string();
                let copy = value.clone();
                let toot_lang = copy["payload"]["language"]
                    .as_str()
                    .expect("redis str")
                    .to_string();

                match (&user.filter, user_langs) {
                    (Filter::Notification, _) if event != "notification" => Ok(Async::NotReady),
                    (Filter::Language, Some(ref langs)) if !langs.contains(&toot_lang) => {
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
