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
use super::receiver::{Receiver, ReceiverErr};
use crate::{
    messages::{CheckedEvent, DynamicEvent, Event},
    parse_client_request::{Stream::Public, Subscription, Timeline},
};
use futures::{
    Async::{self, NotReady, Ready},
    Poll,
};
use std::sync::{Arc, Mutex, MutexGuard};

/// Struct for managing all Redis streams.
#[derive(Clone, Debug)]
pub struct ClientAgent {
    receiver: Arc<Mutex<Receiver>>,
    pub subscription: Subscription,
}

impl ClientAgent {
    pub fn new(receiver: Arc<Mutex<Receiver>>, subscription: &Subscription) -> Self {
        ClientAgent {
            receiver,
            subscription: subscription.clone(),
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
    pub fn subscribe(&mut self) {
        let mut receiver = self.lock_receiver();
        receiver
            .add_subscription(&self.subscription)
            .unwrap_or_else(|e| log::error!("Could not subscribe to the Redis channel: {}", e))
    }

    fn lock_receiver(&self) -> MutexGuard<Receiver> {
        match self.receiver.lock() {
            Ok(inner) => inner,
            Err(e) => {
                log::error!(
                    "Another thread crashed: {}\n
                     Attempting to continue, possibly with invalid data",
                    e
                );
                e.into_inner()
            }
        }
    }
}

/// The stream that the `ClientAgent` manages.  `Poll` is the only method implemented.
impl futures::stream::Stream for ClientAgent {
    type Item = Event;
    type Error = ReceiverErr;

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
            let mut receiver = self.lock_receiver();
            receiver.poll_for(self.subscription.id, self.subscription.timeline)
        };

        let allowed_langs = &self.subscription.allowed_langs;
        let blocked_users = &self.subscription.blocks.blocked_users;
        let blocking_users = &self.subscription.blocks.blocking_users;
        let blocked_domains = &self.subscription.blocks.blocked_domains;
        let (send, block) = (|msg| Ok(Ready(Some(msg))), Ok(NotReady));

        use CheckedEvent::*;
        match result {
            Ok(Async::Ready(Some(event))) => {
                let event = match serde_json::from_str(&event) {
                    Ok(checked_event) => Event::TypeSafe(checked_event),
                    Err(e) => {
                        log::error!(
                            "Error safely parsing Redis input.\
  Mastodon and Flodgatt do not \
                             strictly conform to the same \
version of Mastodon's API.\n{}",
                            e
                        );
                        let dyn_event: DynamicEvent = serde_json::from_str(&event).expect("TODO");
                        Event::Dynamic(dyn_event)
                    }
                };

                match event {
                    Event::TypeSafe(Update { payload: s, .. }) => {
                        match self.subscription.timeline {
                            Timeline(Public, _, _) if s.language_not(allowed_langs) => block,
                            _ if s.involves_any(blocked_users, blocked_domains, blocking_users) => {
                                block
                            }
                            _ => send(Event::TypeSafe(Update {
                                payload: s,
                                queued_at: None,
                            })),
                        }
                    }
                    Event::TypeSafe(_) => send(event),
                    Event::Dynamic(e) if e.event == "update" => match self.subscription.timeline {
                        Timeline(Public, _, _) if e.language_not(allowed_langs) => block,
                        _ if e.involves_any(blocked_users, blocked_domains, blocking_users) => {
                            block
                        }
                        _ => send(Event::Dynamic(e)),
                    },
                    Event::Dynamic(_) => send(event),
                }
            }
            Ok(Ready(None)) => Ok(Ready(None)),
            Ok(NotReady) => Ok(NotReady),
            Err(e) => Err(e),
        }
    }
}
