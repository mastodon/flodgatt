//! Manage all existing Redis PubSub connection
use crate::receiver::Receiver;
use crate::user::User;
use futures::stream::Stream;
use futures::{Async, Poll};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::Error;
use uuid::Uuid;

/// Struct for manageing all Redis streams
#[derive(Clone, Debug)]
pub struct StreamManager {
    receiver: Arc<Mutex<Receiver>>,
    //subscriptions: Arc<Mutex<HashMap<String, Instant>>>,
    id: uuid::Uuid,
    current_user: Option<User>,
}
impl StreamManager {
    pub fn new(reciever: Receiver) -> Self {
        StreamManager {
            receiver: Arc::new(Mutex::new(reciever)),
            //       subscriptions: Arc::new(Mutex::new(HashMap::new())),
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
        println!("ADD lock");
        let mut receiver = self.receiver.lock().unwrap();
        receiver.set_manager_id(self.id);
        receiver.subscribe(timeline);
        dbg!(&receiver);

        println!("ADD unlock");
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
        let result = match receiver.poll() {
            Ok(Async::Ready(Some(value))) => {
                let user = self.clone().current_user.unwrap();

                let user_langs = user.langs.clone();
                let copy = value.clone();
                let event = copy["event"].as_str().unwrap();
                let copy = value.clone();
                let payload = copy["payload"].to_string();
                let copy = value.clone();
                let toot_lang = copy["payload"]["language"]
                    .as_str()
                    .expect("redis str")
                    .to_string();

                println!("sending: {:?}", &payload);
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
        };
        //        dbg!(&result);
        result
    }
}

// CUT FROM .add
//  let mut subscriptions = self.subscriptions.lo ck().expect("No other thread panic");
// subscriptions
//     .entry(timeline.to_string())
//     .or_insert_with(|| {
//         println!("Inserting TL: {}", &timeline);
//***** //
//         Instant::now()
//     });

//        self.current_stream = timeline.to_string();
// // Unsubscribe from that haven't been polled in the last 30 seconds
// let channels = subscriptions.clone();
// let channels_to_unsubscribe = channels
//     .iter()
//     .filter(|(_, time)| time.elapsed().as_secs() > 30);
// for (channel, _) in channels_to_unsubscribe {
//***** //     receiver.unsubscribe(&channel);
// }
// // Update our map of streams
// *subscriptions = channels
//     .clone()
//     .into_iter()
//     .filter(|(_, time)| time.elapsed().as_secs() < 30)
//     .collect();
