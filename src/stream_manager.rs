//! The `StreamManager` is responsible to providing an interface between the `Warp`
//! filters and the underlying mechanics of talking with Redis/managing multiple
//! threads.  The `StreamManager` is the only struct that any Warp code should
//! need to communicate with.
//!
//! The `StreamManager`'s interface is very simple.  All you can do with it is:
//!  * Create a totally new `StreamManger` with no shared data;
//!  * Assign an existing `StreamManager` to manage an new timeline/user pair; or
//!  * Poll an existing `StreamManager` to see if there are any new messages
//!    for clients
//!
//! When you poll the `StreamManager`, it is responsible for polling internal data
//! structures, getting any updates from Redis, and then filtering out any updates
//! that should be excluded by relevant filters.
//!
//! Because `StreamManagers` are lightweight data structures that do not directly
//! communicate with Redis, it is appropriate to create a new `StreamManager` for
//! each new client connection.
use crate::{
    receiver::Receiver,
    user::{Filter, User},
};
use futures::{Async, Poll};
use serde_json::{json, Value};
use std::sync;
use std::time;
use tokio::io::Error;
use uuid::Uuid;

/// Struct for managing all Redis streams.
#[derive(Clone, Default, Debug)]
pub struct StreamManager {
    receiver: sync::Arc<sync::Mutex<Receiver>>,
    id: uuid::Uuid,
    target_timeline: String,
    current_user: User,
}

impl StreamManager {
    /// Create a new `StreamManager` with no shared data.
    pub fn new() -> Self {
        StreamManager {
            receiver: sync::Arc::new(sync::Mutex::new(Receiver::new())),
            id: Uuid::default(),
            target_timeline: String::new(),
            current_user: User::public(),
        }
    }

    /// Assign the `StreamManager` to manage a new timeline/user pair.
    ///
    /// Note that this *may or may not* result in a new Redis connection.
    /// If the server has already subscribed to the timeline on behalf of
    /// a different user, the `StreamManager` is responsible for figuring
    /// that out and avoiding duplicated connections.  Thus, it is safe to
    /// use this method for each new client connection.
    pub fn manage_new_timeline(&self, target_timeline: &str, user: User) -> Self {
        let manager_id = Uuid::new_v4();
        let mut receiver = self.receiver.lock().expect("No thread panic (stream.rs)");
        receiver.manage_new_timeline(manager_id, target_timeline);
        StreamManager {
            id: manager_id,
            current_user: user,
            target_timeline: target_timeline.to_owned(),
            receiver: self.receiver.clone(),
        }
    }
}

/// The stream that the `StreamManager` manages.  `Poll` is the only method implemented.
impl futures::stream::Stream for StreamManager {
    type Item = Value;
    type Error = Error;

    /// Checks for any new messages that should be sent to the client.
    ///
    /// The `StreamManager` will poll underlying data structures and will reply
    /// with an `Ok(Ready(Some(Value)))` if there is a new message to send to
    /// the client.  If there is no new message or if the new message should be
    /// filtered out based on one of the user's filters, then the `StreamManager`
    /// will reply with `Ok(NotReady)`.  The `StreamManager` will buble up any
    /// errors from the underlying data structures.
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let start_time = time::Instant::now();
        let result = {
            let mut receiver = self
                .receiver
                .lock()
                .expect("StreamManager: No other thread panic");
            receiver.configure_for_polling(self.id, &self.target_timeline.clone());
            receiver.poll()
        };
        println!("Polling took: {:?}", start_time.elapsed());
        let result = match result {
            Ok(Async::Ready(Some(value))) => {
                let user_langs = self.current_user.langs.clone();
                let toot = Toot::from_json(value);
                toot.ignore_if_caught_by_filter(&self.current_user.filter, user_langs)
            }
            Ok(inner_value) => Ok(inner_value),
            Err(e) => Err(e),
        };
        result
    }
}

struct Toot {
    category: String,
    payload: String,
    language: String,
}
impl Toot {
    fn from_json(value: Value) -> Self {
        Self {
            category: value["event"].as_str().expect("Redis string").to_owned(),
            payload: value["payload"].to_string(),
            language: value["payload"]["language"]
                .as_str()
                .expect("Redis str")
                .to_string(),
        }
    }

    fn to_optional_json(&self) -> Option<Value> {
        Some(json!(
            {"event": self.category,
             "payload": self.payload,}
        ))
    }

    fn ignore_if_caught_by_filter(
        &self,
        filter: &Filter,
        user_langs: Option<Vec<String>>,
    ) -> Result<Async<Option<Value>>, Error> {
        let toot = self;

        let (send_msg, skip_msg) = (
            Ok(Async::Ready(toot.to_optional_json())),
            Ok(Async::NotReady),
        );

        match &filter {
            Filter::NoFilter => send_msg,
            Filter::Notification if toot.category == "notification" => send_msg,
            // If not, skip it
            Filter::Notification => skip_msg,
            Filter::Language if user_langs.is_none() => send_msg,
            Filter::Language if user_langs.expect("").contains(&toot.language) => send_msg,
            // If not, skip it
            Filter::Language => skip_msg,
        }
    }
}
