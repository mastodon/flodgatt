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
    pub fn add_subscription(&mut self, subscription: &Subscription) -> Result<()> {
        let (tag, tl) = (subscription.hashtag_name.clone(), subscription.timeline);

        if let (Some(hashtag), Timeline(Stream::Hashtag(id), _, _)) = (tag, tl) {
            self.redis_connection.update_cache(hashtag, id);
        };
        self.msg_queues.insert(subscription.id, MsgQueue::new(tl));

        let number_of_subscriptions = self
            .clients_per_timeline
            .entry(tl)
            .and_modify(|n| *n += 1)
            .or_insert(1);

        use RedisCmd::*;
        if *number_of_subscriptions == 1 {
            self.redis_connection.send_cmd(Subscribe, &tl)?
        };

        Ok(())
    }

    pub fn remove_subscription(&mut self, subscription: &Subscription) -> Result<()> {
        let tl = subscription.timeline;
        self.msg_queues.remove(&subscription.id);
        let number_of_subscriptions = self
            .clients_per_timeline
            .entry(tl)
            .and_modify(|n| *n -= 1)
            .or_insert_with(|| {
                log::error!(
                    "Attempted to unsubscribe from a timeline to which you were not subscribed: {:?}",
                    tl
                );
                0
            });
        use RedisCmd::*;
        if *number_of_subscriptions == 0 {
            self.redis_connection.send_cmd(Unsubscribe, &tl)?;
            self.clients_per_timeline.remove_entry(&tl);
        };

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
                Ok(Async::Ready(Some((timeline, event)))) => {
                    self.msg_queues
                        .values_mut()
                        .filter(|msg_queue| msg_queue.timeline == timeline)
                        .for_each(|msg_queue| {
                            msg_queue.messages.push_back(event.clone());
                        });
                }
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(None)) => (),
                Err(err) => Err(err)?,
            }
        }

        // If the `msg_queue` being polled has any new messages, return the first (oldest) one
        match self.msg_queues.get_mut(&id) {
            Some(msg_q) => match msg_q.messages.pop_front() {
                Some(event) => Ok(Async::Ready(Some(event))),
                None => Ok(Async::NotReady),
            },
            None => {
                log::error!("Polled a MsgQueue that had not been set up.  Setting it up now.");
                self.msg_queues.insert(id, MsgQueue::new(timeline));
                Ok(Async::NotReady)
            }
        }
    }

    pub fn count_connections(&self) -> String {
        format!(
            "Current connections: {}",
            self.clients_per_timeline.values().sum::<i32>()
        )
    }

    pub fn list_connections(&self) -> String {
        let max_len = self
            .clients_per_timeline
            .keys()
            .fold(0, |acc, el| acc.max(format!("{:?}:", el).len()));
        self.clients_per_timeline
            .iter()
            .map(|(tl, n)| {
                let tl_txt = format!("{:?}:", tl);
                format!("{:>1$} {2}\n", tl_txt, max_len, n)
            })
            .collect()
    }

    pub fn queue_length(&self) -> String {
        format!(
            "Longest MessageQueue: {}",
            self.msg_queues
                .0
                .values()
                .fold(0, |acc, el| acc.max(el.messages.len()))
        )
    }
}
