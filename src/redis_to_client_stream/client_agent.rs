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
use super::receiver::Receiver;
use crate::{
    config,
    messages::Event,
    parse_client_request::{Stream::Public, Subscription, Timeline},
};
use futures::{
    Async::{self, NotReady, Ready},
    Poll,
};
use std::sync::{Arc, Mutex};
use tokio::io::Error;
use uuid::Uuid;

/// Struct for managing all Redis streams.
#[derive(Clone, Debug)]
pub struct ClientAgent {
    receiver: Arc<Mutex<Receiver>>,
    id: Uuid,
    pub subscription: Subscription,
}

impl ClientAgent {
    /// Create a new `ClientAgent` with no shared data.
    pub fn blank(redis_cfg: config::RedisConfig) -> Self {
        ClientAgent {
            receiver: Arc::new(Mutex::new(Receiver::new(redis_cfg))),
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
        use std::time::Instant;
        self.id = Uuid::new_v4();
        self.subscription = subscription;
        let start_time = Instant::now();
        let mut receiver = self.receiver.lock().expect("No thread panic (stream.rs)");
        receiver.manage_new_timeline(
            self.id,
            self.subscription.timeline,
            self.subscription.hashtag_name.clone(),
        );
        log::info!("init_for_user had lock for: {:?}", start_time.elapsed());
    }
}

/// The stream that the `ClientAgent` manages.  `Poll` is the only method implemented.
impl futures::stream::Stream for ClientAgent {
    type Item = Event;
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
        let result = {
            let mut receiver = self
                .receiver
                .lock()
                .expect("ClientAgent: No other thread panic");
            receiver.configure_for_polling(self.id, self.subscription.timeline);
            receiver.poll()
        };

        let allowed_langs = &self.subscription.allowed_langs;
        let blocked_users = &self.subscription.blocks.blocked_users;
        let blocking_users = &self.subscription.blocks.blocking_users;
        let blocked_domains = &self.subscription.blocks.blocked_domains;
        let (send, block) = (|msg| Ok(Ready(Some(msg))), Ok(NotReady));
        use Event::*;
        match result {
            Ok(Async::Ready(Some(event))) => match event {
                Update {
                    payload: status, ..
                } => match self.subscription.timeline {
                    _ if status.involves_blocked_user(blocked_users) => block,
                    _ if status.from_blocked_domain(blocked_domains) => block,
                    _ if status.from_blocking_user(blocking_users) => block,
                    Timeline(Public, _, _) if status.language_not_allowed(allowed_langs) => block,
                    _ => send(Update {
                        payload: status,
                        queued_at: None,
                    }),
                },
                Notification { .. }
                | Conversation { .. }
                | Delete { .. }
                | FiltersChanged
                | Announcement { .. }
                | AnnouncementReaction { .. }
                | AnnouncementDelete { .. } => send(event),
            },
            Ok(Ready(None)) => Ok(Ready(None)),
            Ok(NotReady) => Ok(NotReady),
            Err(e) => Err(e),
        }
    }
}
