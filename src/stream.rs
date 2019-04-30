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
#[derive(Clone)]
pub struct StreamManager {
    receiver: Arc<Mutex<Receiver>>,
    subscriptions: Arc<Mutex<HashMap<String, Instant>>>,
    current_stream: String,
    id: uuid::Uuid,
}
impl StreamManager {
    pub fn new(reciever: Receiver) -> Self {
        StreamManager {
            receiver: Arc::new(Mutex::new(reciever)),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            current_stream: String::new(),
            id: Uuid::new_v4(),
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
    pub fn add(&mut self, timeline: &str, _user: &User) -> &Self {
        let mut subscriptions = self.subscriptions.lock().expect("No other thread panic");
        let mut receiver = self.receiver.lock().unwrap();
        subscriptions
            .entry(timeline.to_string())
            .or_insert_with(|| {
                receiver.subscribe(timeline);
                Instant::now()
            });

        // Unsubscribe from that haven't been polled in the last 30 seconds
        let channels = subscriptions.clone();
        let channels_to_unsubscribe = channels
            .iter()
            .filter(|(_, time)| time.elapsed().as_secs() > 30);
        for (channel, _) in channels_to_unsubscribe {
            receiver.unsubscribe(&channel);
        }
        // Update our map of streams
        *subscriptions = channels
            .clone()
            .into_iter()
            .filter(|(_, time)| time.elapsed().as_secs() > 30)
            .collect();

        self.current_stream = timeline.to_string();
        self
    }
}
impl Stream for StreamManager {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut subscriptions = self.subscriptions.lock().expect("No other thread panic");
        let target_stream = self.current_stream.clone();
        subscriptions.insert(target_stream.clone(), Instant::now());

        let mut receiver = self.receiver.lock().expect("No other thread panic");
        receiver.set_polled_by(self.id);

        match receiver.poll() {
            Ok(Async::Ready(Some(value))) => Ok(Async::Ready(Some(value))),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
