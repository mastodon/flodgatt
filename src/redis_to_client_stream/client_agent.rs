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
//! each new client connection (each in its own thread).use super::{message::Message, receiver::Receiver}
use super::{message::Message, receiver::Receiver};
use crate::{
    config,
    parse_client_request::user::{PgPool, Stream::Public, Subscription, Timeline},
};
use futures::{
    Async::{self, NotReady, Ready},
    Poll,
};

use std::sync;
use tokio::io::Error;
use uuid::Uuid;

/// Struct for managing all Redis streams.
#[derive(Clone, Debug)]
pub struct ClientAgent {
    receiver: sync::Arc<sync::Mutex<Receiver>>,
    id: uuid::Uuid,
    subscription: Subscription,
}

impl ClientAgent {
    /// Create a new `ClientAgent` with no shared data.
    pub fn blank(redis_cfg: config::RedisConfig, pg_pool: PgPool) -> Self {
        ClientAgent {
            receiver: sync::Arc::new(sync::Mutex::new(Receiver::new(redis_cfg, pg_pool))),
            id: Uuid::default(),
            subscription: Subscription::default(),
        }
    }

    /// Clones the `ClientAgent`, sharing the `Receiver`.
    pub fn clone_with_shared_receiver(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            id: self.id,
            subscription: self.subscription.clone(),
        }
    }

    /// Initializes the `ClientAgent` with a unique ID associated with a specific user's
    /// subscription.  Also passes values to the `Receiver` for it's initialization.
    ///
    /// Note that this *may or may not* result in a new Redis connection.
    /// If the server has already subscribed to the timeline on behalf of
    /// a different user, the `Receiver` is responsible for figuring
    /// that out and avoiding duplicated connections.  Thus, it is safe to
    /// use this method for each new client connection.
    pub fn init_for_user(&mut self, subscription: Subscription) {
        self.id = Uuid::new_v4();
        self.subscription = subscription;
        let mut receiver = self.receiver.lock().expect("No thread panic (stream.rs)");
        receiver.manage_new_timeline(self.id, self.subscription.timeline);
    }
}

/// The stream that the `ClientAgent` manages.  `Poll` is the only method implemented.
impl futures::stream::Stream for ClientAgent {
    type Item = Message;
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
            receiver.configure_for_polling(self.id, self.subscription.timeline);
            receiver.poll()
        };
        if start_time.elapsed().as_millis() > 1 {
            log::warn!("Polling the Receiver took: {:?}", start_time.elapsed());
        };

        let allowed_langs = &self.subscription.allowed_langs;
        let blocked_users = &self.subscription.blocks.blocked_users;
        let blocking_users = &self.subscription.blocks.blocking_users;
        let blocked_domains = &self.subscription.blocks.blocked_domains;
        let (send, block) = (|msg| Ok(Ready(Some(msg))), Ok(NotReady));
        use Message::*;
        match result {
            Ok(Async::Ready(Some(json))) => match Message::from_json(json) {
                Update(status) => match self.subscription.timeline {
                    _ if status.involves_blocked_user(blocked_users) => block,
                    _ if status.from_blocked_domain(blocked_domains) => block,
                    _ if status.from_blocking_user(blocking_users) => block,
                    Timeline(Public, _, _) if status.language_not_allowed(allowed_langs) => block,
                    _ => send(Update(status)),
                },
                Notification(payload) => send(Notification(payload)),
                Conversation(payload) => send(Conversation(payload)),
                Delete(status_id) => send(Delete(status_id)),
                FiltersChanged => send(FiltersChanged),
                Announcement(content) => send(Announcement(content)),
                UnknownEvent(event, payload) => send(UnknownEvent(event, payload)),
            },
            Ok(Ready(None)) => Ok(Ready(None)),
            Ok(NotReady) => Ok(NotReady),
            Err(e) => Err(e),
        }
    }
}
