//! Provides an interface between the `Warp` filters and the underlying
//! mechanics of talking with Redis/managing multiple threads.
//!
//! The `ClientAgent`'s interface is very simple.  All you can do with it is:
//!  * Create a totally new `ClientAgent` with no shared data;
//!  * Clone an existing `ClientAgent`, sharing the `Receiver`;
//!  * Manage an new timeline/user pair; or
//!  * Poll an existing `ClientAgent` to see if there are any new messages
//!    for clients
//!
//! When you poll the `ClientAgent`, it is responsible for polling internal data
//! structures, getting any updates from Redis, and then filtering out any updates
//! that should be excluded by relevant filters.
//!
//! Because `StreamManagers` are lightweight data structures that do not directly
//! communicate with Redis, it we create a new `ClientAgent` for
//! each new client connection (each in its own thread).
use super::receiver::Receiver;
use crate::{config, parse_client_request::user::User};
use futures::{Async, Poll};
use serde_json::Value;
use std::{collections::HashSet, sync};
use tokio::io::Error;
use uuid::Uuid;

/// Struct for managing all Redis streams.
#[derive(Clone, Debug)]
pub struct ClientAgent {
    receiver: sync::Arc<sync::Mutex<Receiver>>,
    id: uuid::Uuid,
    pub target_timeline: String,
    pub current_user: User,
}

impl ClientAgent {
    /// Create a new `ClientAgent` with no shared data.
    pub fn blank(redis_cfg: config::RedisConfig) -> Self {
        ClientAgent {
            receiver: sync::Arc::new(sync::Mutex::new(Receiver::new(redis_cfg))),
            id: Uuid::default(),
            target_timeline: String::new(),
            current_user: User::default(),
        }
    }

    /// Clones the `ClientAgent`, sharing the `Receiver`.
    pub fn clone_with_shared_receiver(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            id: self.id,
            target_timeline: self.target_timeline.clone(),
            current_user: self.current_user.clone(),
        }
    }
    /// Initializes the `ClientAgent` with a unique ID, a `User`, and the target timeline.
    /// Also passes values to the `Receiver` for it's initialization.
    ///
    /// Note that this *may or may not* result in a new Redis connection.
    /// If the server has already subscribed to the timeline on behalf of
    /// a different user, the `Receiver` is responsible for figuring
    /// that out and avoiding duplicated connections.  Thus, it is safe to
    /// use this method for each new client connection.
    pub fn init_for_user(&mut self, user: User) {
        self.id = Uuid::new_v4();
        self.target_timeline = user.target_timeline.to_owned();
        self.current_user = user;
        let mut receiver = self.receiver.lock().expect("No thread panic (stream.rs)");
        receiver.manage_new_timeline(self.id, &self.target_timeline);
    }
}

/// The stream that the `ClientAgent` manages.  `Poll` is the only method implemented.
impl futures::stream::Stream for ClientAgent {
    type Item = Toot;
    type Error = Error;

    /// Checks for any new messages that should be sent to the client.
    ///
    /// The `ClientAgent` polls the `Receiver` and replies
    /// with `Ok(Ready(Some(Value)))` if there is a new message to send to
    /// the client.  If there is no new message or if the new message should be
    /// filtered out based on one of the user's filters, then the `ClientAgent`
    /// replies with `Ok(NotReady)`.  The `ClientAgent` bubles up any
    /// errors from the underlying data structures.
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let start_time = std::time::Instant::now();
        let result = {
            let mut receiver = self
                .receiver
                .lock()
                .expect("ClientAgent: No other thread panic");
            receiver.configure_for_polling(self.id, &self.target_timeline.clone());
            receiver.poll()
        };
        if start_time.elapsed().as_millis() > 1 {
            log::warn!("Polling the Receiver took: {:?}", start_time.elapsed());
        };

        match result {
            Ok(Async::Ready(Some(value))) => {
                let user = &self.current_user;
                let toot = Toot::from_json(value);
                toot.filter(&user)
            }
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

/// The message to send to the client (which might not literally be a toot in some cases).
#[derive(Debug, Clone)]
pub struct Toot {
    pub category: String,
    pub payload: Value,
    pub language: Option<String>,
}

impl Toot {
    /// Construct a `Toot` from well-formed JSON.
    pub fn from_json(value: Value) -> Self {
        let category = value["event"].as_str().expect("Redis string").to_owned();
        let language = if category == "update" {
            Some(value["payload"]["language"].to_string())
        } else {
            None
        };

        Self {
            category,
            payload: value["payload"].clone(),
            language,
        }
    }

    pub fn get_originating_domain(&self) -> HashSet<String> {
        let api = "originating  Invariant Violation: JSON value does not conform to Mastdon API";
        let mut originating_domain = HashSet::new();
        originating_domain.insert(
            self.payload["account"]["acct"]
                .as_str()
                .expect(&api)
                .split("@")
                .nth(1)
                .expect(&api)
                .to_string(),
        );
        originating_domain
    }

    pub fn get_involved_users(&self) -> HashSet<i64> {
        let mut involved_users: HashSet<i64> = HashSet::new();
        let msg = self.payload.clone();

        let api = "Invariant Violation: JSON value does not conform to Mastdon API";
        involved_users.insert(msg["account"]["id"].str_to_i64().expect(&api));
        if let Some(mentions) = msg["mentions"].as_array() {
            for mention in mentions {
                involved_users.insert(mention["id"].str_to_i64().expect(&api));
            }
        }
        if let Some(replied_to_account) = msg["in_reply_to_account_id"].as_str() {
            involved_users.insert(replied_to_account.parse().expect(&api));
        }

        if let Some(reblog) = msg["reblog"].as_object() {
            involved_users.insert(reblog["account"]["id"].str_to_i64().expect(&api));
        }
        involved_users
    }

    /// Filter out any `Toot`'s that fail the provided filter.
    pub fn filter(self, user: &User) -> Result<Async<Option<Self>>, Error> {
        let toot = self;

        let category = toot.category.clone();
        let toot_language = &toot.language.clone().expect("Valid lanugage");
        let (send_msg, skip_msg) = (Ok(Async::Ready(Some(toot))), Ok(Async::NotReady));

        if category == "update" {
            use crate::parse_client_request::user::Filter;

            match &user.filter {
                Filter::NoFilter => send_msg,
                Filter::Notification if category == "notification" => send_msg,
                // If not, skip it
                Filter::Notification => skip_msg,
                Filter::Language if user.langs.is_none() => send_msg,
                Filter::Language if user.langs.clone().expect("").contains(toot_language) => {
                    send_msg
                }
                // If not, skip it
                Filter::Language => skip_msg,
            }
        } else {
            send_msg
        }
    }
}

trait ConvertValue {
    fn str_to_i64(&self) -> Result<i64, Box<dyn std::error::Error>>;
}

impl ConvertValue for Value {
    fn str_to_i64(&self) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(self
            .as_str()
            .ok_or(format!("{} is not a string", &self))?
            .parse()
            .map_err(|_| "Could not parse str")?)
    }
}
