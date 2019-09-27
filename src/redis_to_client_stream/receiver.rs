//! Receives data from Redis, sorts it by `ClientAgent`, and stores it until
//! polled by the correct `ClientAgent`.  Also manages sububscriptions and
//! unsubscriptions to/from Redis.
use super::{redis_cmd, redis_stream};
use crate::{config, pubsub_cmd};
use futures::{Async, Poll};
use log::info;
use serde_json::Value;
use std::{collections, io::Write, net, time};
use tokio::io::Error;
use uuid::Uuid;

/// The item that streams from Redis and is polled by the `ClientAgent`
#[derive(Debug)]
pub struct Receiver {
    pub pubsub_connection: net::TcpStream,
    secondary_redis_connection: net::TcpStream,
    redis_polled_at: time::Instant,
    timeline: String,
    manager_id: Uuid,
    pub msg_queues: collections::HashMap<Uuid, MsgQueue>,
    clients_per_timeline: collections::HashMap<String, i32>,
    pub incoming_raw_msg: String,
}

impl Receiver {
    /// Create a new `Receiver`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn new() -> Self {
        let (pubsub_connection, secondary_redis_connection) = config::redis_addr();
        Self {
            pubsub_connection,
            secondary_redis_connection,
            redis_polled_at: time::Instant::now(),
            timeline: String::new(),
            manager_id: Uuid::default(),
            msg_queues: collections::HashMap::new(),
            clients_per_timeline: collections::HashMap::new(),
            /// The unprocessed message from Redis, consisting of 0 or more
            /// actual `messages` in the sense of updates to send.
            incoming_raw_msg: String::new(),
        }
    }

    /// Assigns the `Receiver` a new timeline to monitor and runs other
    /// first-time setup.
    ///
    /// Note: this method calls `subscribe_or_unsubscribe_as_needed`,
    /// so Redis PubSub subscriptions are only updated when a new timeline
    /// comes under management for the first time.
    pub fn manage_new_timeline(&mut self, manager_id: Uuid, timeline: &str) {
        self.manager_id = manager_id;
        self.timeline = timeline.to_string();
        self.msg_queues
            .insert(self.manager_id, MsgQueue::new(timeline));
        self.subscribe_or_unsubscribe_as_needed(timeline);
    }

    /// Set the `Receiver`'s manager_id and target_timeline fields to the appropriate
    /// value to be polled by the current `StreamManager`.
    pub fn configure_for_polling(&mut self, manager_id: Uuid, timeline: &str) {
        self.manager_id = manager_id;
        self.timeline = timeline.to_string();
    }

    /// Drop any PubSub subscriptions that don't have active clients and check
    /// that there's a subscription to the current one.  If there isn't, then
    /// subscribe to it.
    fn subscribe_or_unsubscribe_as_needed(&mut self, timeline: &str) {
        let mut timelines_to_modify = Vec::new();
        struct Change {
            timeline: String,
            change_in_subscriber_number: i32,
        }

        timelines_to_modify.push(Change {
            timeline: timeline.to_owned(),
            change_in_subscriber_number: 1,
        });

        // Keep only message queues that have been polled recently
        self.msg_queues.retain(|_id, msg_queue| {
            if msg_queue.last_polled_at.elapsed() < time::Duration::from_secs(30) {
                true
            } else {
                let timeline = &msg_queue.redis_channel;
                timelines_to_modify.push(Change {
                    timeline: timeline.to_owned(),
                    change_in_subscriber_number: -1,
                });
                false
            }
        });

        // Record the lower number of clients subscribed to that channel
        for change in timelines_to_modify {
            let mut need_to_subscribe = false;
            let count_of_subscribed_clients = self
                .clients_per_timeline
                .entry(change.timeline.clone())
                .and_modify(|n| *n += change.change_in_subscriber_number)
                .or_insert_with(|| {
                    need_to_subscribe = true;
                    1
                });
            // If no clients, unsubscribe from the channel
            if *count_of_subscribed_clients <= 0 {
                pubsub_cmd!("unsubscribe", self, change.timeline.clone());
            }
            if need_to_subscribe {
                pubsub_cmd!("subscribe", self, change.timeline.clone());
            }
        }
    }

    fn log_number_of_msgs_in_queue(&self) {
        let messages_waiting = self
            .msg_queues
            .get(&self.manager_id)
            .expect("Guaranteed by match block")
            .messages
            .len();
        match messages_waiting {
            number if number > 10 => {
                log::error!("{} messages waiting in the queue", messages_waiting)
            }
            _ => log::info!("{} messages waiting in the queue", messages_waiting),
        }
    }

    fn get_target_msg_queue(&mut self) -> collections::hash_map::Entry<Uuid, MsgQueue> {
        self.msg_queues.entry(self.manager_id)
    }
}

impl Default for Receiver {
    fn default() -> Self {
        Receiver::new()
    }
}

/// The stream that the ClientAgent polls to learn about new messages.
impl futures::stream::Stream for Receiver {
    type Item = Value;
    type Error = Error;

    /// Returns the oldest message in the `ClientAgent`'s queue (if any).
    ///
    /// Note: This method does **not** poll Redis every time, because polling
    /// Redis is signifiantly more time consuming that simply returning the
    /// message already in a queue.  Thus, we only poll Redis if it has not
    /// been polled lately.
    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let timeline = self.timeline.clone();

        if self.redis_polled_at.elapsed()
            > time::Duration::from_millis(*config::REDIS_POLL_INTERVAL)
        {
            redis_stream::AsyncReadableStream::poll_redis(self);
            self.redis_polled_at = time::Instant::now();
        }

        // Record current time as last polled time
        self.get_target_msg_queue()
            .and_modify(|msg_queue| msg_queue.last_polled_at = time::Instant::now());

        // If the `msg_queue` being polled has any new messages, return the first (oldest) one
        match self
            .get_target_msg_queue()
            .or_insert_with(|| MsgQueue::new(timeline.clone()))
            .messages
            .pop_front()
        {
            Some(value) => {
                self.log_number_of_msgs_in_queue();
                Ok(Async::Ready(Some(value)))
            }
            _ => Ok(Async::NotReady),
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        pubsub_cmd!("unsubscribe", self, self.timeline.clone());
    }
}

#[derive(Debug, Clone)]
pub struct MsgQueue {
    pub messages: collections::VecDeque<Value>,
    last_polled_at: time::Instant,
    pub redis_channel: String,
}

impl MsgQueue {
    fn new(redis_channel: impl std::fmt::Display) -> Self {
        let redis_channel = redis_channel.to_string();
        MsgQueue {
            messages: collections::VecDeque::new(),
            last_polled_at: time::Instant::now(),
            redis_channel,
        }
    }
}
