//! Receives data from Redis, sorts it by `ClientAgent`, and stores it until
//! polled by the correct `ClientAgent`.  Also manages sububscriptions and
//! unsubscriptions to/from Redis.
mod err;
mod message_queues;

pub use err::ReceiverErr;
pub use message_queues::{MessageQueues, MsgQueue};

use super::redis::{redis_connection::RedisCmd, RedisConn};

use crate::{
    config,
    messages::Event,
    parse_client_request::{Stream, Subscription, Timeline},
};

use futures::{Async, Poll};
use std::{
    collections::HashMap,
    result,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

type Result<T> = result::Result<T, ReceiverErr>;

/// The item that streams from Redis and is polled by the `ClientAgent`
#[derive(Debug)]
pub struct Receiver {
    redis_connection: RedisConn,
    pub msg_queues: MessageQueues,
    clients_per_timeline: HashMap<Timeline, i32>,
}

impl Receiver {
    /// Create a new `Receiver`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn try_from(redis_cfg: config::RedisConfig) -> Result<Self> {
        let redis_connection = RedisConn::new(redis_cfg)?;

        Ok(Self {
            redis_connection,
            msg_queues: MessageQueues(HashMap::new()),
            clients_per_timeline: HashMap::new(),
        })
    }

    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    /// Assigns the `Receiver` a new timeline to monitor and runs other
    /// first-time setup.
    ///
    /// Note: this method calls `subscribe_or_unsubscribe_as_needed`,
    /// so Redis PubSub subscriptions are only updated when a new timeline
    /// comes under management for the first time.
    pub fn add_subscription(&mut self, subscription: &Subscription) -> Result<()> {
        let (tag, tl) = (subscription.hashtag_name.clone(), subscription.timeline);

        if let (Some(hashtag), Timeline(Stream::Hashtag(id), _, _)) = (tag, tl) {
            self.redis_connection.update_cache(hashtag, id);
        };
        self.msg_queues.insert(subscription.id, MsgQueue::new(tl));
        self.subscribe_or_unsubscribe_as_needed(tl)?;
        Ok(())
    }

    /// Returns the oldest message in the `ClientAgent`'s queue (if any).
    ///
    /// Note: This method does **not** poll Redis every time, because polling
    /// Redis is significantly more time consuming that simply returning the
    /// message already in a queue.  Thus, we only poll Redis if it has not
    /// been polled lately.
    pub fn poll_for(&mut self, id: Uuid, timeline: Timeline) -> Poll<Option<Event>, ReceiverErr> {
        loop {
            match self.redis_connection.poll_redis() {
                Ok(Async::Ready(Some((timeline, event)))) => self
                    .msg_queues
                    .values_mut()
                    .filter(|msg_queue| msg_queue.timeline == timeline)
                    .for_each(|msg_queue| {
                        msg_queue.messages.push_back(event.clone());
                    }),
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(None)) => (),
                Err(err) => Err(err)?,
            }
        }

        // If the `msg_queue` being polled has any new messages, return the first (oldest) one
        match self.msg_queues.get_mut(&id) {
            Some(msg_q) => {
                msg_q.update_polled_at_time();
                match msg_q.messages.pop_front() {
                    Some(event) => Ok(Async::Ready(Some(event))),
                    None => Ok(Async::NotReady),
                }
            }
            None => {
                log::error!("Polled a MsgQueue that had not been set up.  Setting it up now.");
                self.msg_queues.insert(id, MsgQueue::new(timeline));
                Ok(Async::NotReady)
            }
        }
    }

    /// Drop any PubSub subscriptions that don't have active clients and check
    /// that there's a subscription to the current one.  If there isn't, then
    /// subscribe to it.
    fn subscribe_or_unsubscribe_as_needed(&mut self, tl: Timeline) -> Result<()> {
        let timelines_to_modify = self.msg_queues.calculate_timelines_to_add_or_drop(tl);

        // Record the lower number of clients subscribed to that channel
        for change in timelines_to_modify {
            let timeline = change.timeline;

            let count_of_subscribed_clients = self
                .clients_per_timeline
                .entry(timeline)
                .and_modify(|n| *n += change.in_subscriber_number)
                .or_insert_with(|| 1);

            // If no clients, unsubscribe from the channel
            use RedisCmd::*;
            if *count_of_subscribed_clients <= 0 {
                self.redis_connection.send_cmd(Unsubscribe, &timeline)?;
            } else if *count_of_subscribed_clients == 1 && change.in_subscriber_number == 1 {
                self.redis_connection.send_cmd(Subscribe, &timeline)?
            }
        }
        Ok(())
    }
}
