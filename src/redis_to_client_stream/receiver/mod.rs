//! Receives data from Redis, sorts it by `ClientAgent`, and stores it until
//! polled by the correct `ClientAgent`.  Also manages sububscriptions and
//! unsubscriptions to/from Redis.
mod message_queues;

pub use message_queues::{MessageQueues, MsgQueue};

use crate::{
    config,
    err::RedisParseErr,
    messages::Event,
    parse_client_request::{Stream, Timeline},
    redis_to_client_stream::redis::RedisConn,
};
use futures::{Async, Poll};
use lru::LruCache;
use std::{collections::HashMap, time::Instant};
use uuid::Uuid;

/// The item that streams from Redis and is polled by the `ClientAgent`
#[derive(Debug)]
pub struct Receiver {
    redis_connection: RedisConn,
    timeline: Timeline,
    manager_id: Uuid,
    pub msg_queues: MessageQueues,
    clients_per_timeline: HashMap<Timeline, i32>,
    hashtag_cache: LruCache<i64, String>,
    // TODO: eventually, it might make sense to have Mastodon publish to timelines with
    //       the tag number instead of the tag name.  This would save us from dealing
    //       with a cache here and would be consistent with how lists/users are handled.
}

impl Receiver {
    /// Create a new `Receiver`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn new(redis_cfg: config::RedisConfig) -> Self {
        let redis_connection = RedisConn::new(redis_cfg);

        Self {
            redis_connection,
            timeline: Timeline::empty(),
            manager_id: Uuid::default(),
            msg_queues: MessageQueues(HashMap::new()),
            clients_per_timeline: HashMap::new(),
            hashtag_cache: LruCache::new(1000),
            // should this be a run-time option?
        }
    }

    /// Assigns the `Receiver` a new timeline to monitor and runs other
    /// first-time setup.
    ///
    /// Note: this method calls `subscribe_or_unsubscribe_as_needed`,
    /// so Redis PubSub subscriptions are only updated when a new timeline
    /// comes under management for the first time.
    pub fn manage_new_timeline(&mut self, id: Uuid, tl: Timeline, hashtag: Option<String>) {
        self.timeline = tl;
        if let (Some(hashtag), Timeline(Stream::Hashtag(id), _, _)) = (hashtag, tl) {
            self.hashtag_cache.put(id, hashtag.clone());
            self.redis_connection.update_cache(hashtag, id);
        };

        self.msg_queues.insert(id, MsgQueue::new(tl));
        self.subscribe_or_unsubscribe_as_needed(tl);
    }

    /// Set the `Receiver`'s manager_id and target_timeline fields to the appropriate
    /// value to be polled by the current `StreamManager`.
    pub fn configure_for_polling(&mut self, manager_id: Uuid, timeline: Timeline) {
        self.manager_id = manager_id;
        self.timeline = timeline;
    }

    /// Drop any PubSub subscriptions that don't have active clients and check
    /// that there's a subscription to the current one.  If there isn't, then
    /// subscribe to it.
    fn subscribe_or_unsubscribe_as_needed(&mut self, timeline: Timeline) {
        let start_time = Instant::now();
        let timelines_to_modify = self.msg_queues.calculate_timelines_to_add_or_drop(timeline);

        // Record the lower number of clients subscribed to that channel
        for change in timelines_to_modify {
            let timeline = change.timeline;
            let hashtag = match timeline {
                Timeline(Stream::Hashtag(id), _, _) => self.hashtag_cache.get(&id),
                _non_hashtag_timeline => None,
            };

            let count_of_subscribed_clients = self
                .clients_per_timeline
                .entry(timeline)
                .and_modify(|n| *n += change.in_subscriber_number)
                .or_insert_with(|| 1);

            // If no clients, unsubscribe from the channel
            if *count_of_subscribed_clients <= 0 {
                self.redis_connection
                    .send_unsubscribe_cmd(&timeline.to_redis_raw_timeline(hashtag));
            } else if *count_of_subscribed_clients == 1 && change.in_subscriber_number == 1 {
                self.redis_connection
                    .send_subscribe_cmd(&timeline.to_redis_raw_timeline(hashtag));
            }
        }
        if start_time.elapsed().as_millis() > 1 {
            log::warn!("Sending cmd to Redis took: {:?}", start_time.elapsed());
        };
    }
}

/// The stream that the ClientAgent polls to learn about new messages.
impl futures::stream::Stream for Receiver {
    type Item = Event;
    type Error = RedisParseErr;

    /// Returns the oldest message in the `ClientAgent`'s queue (if any).
    ///
    /// Note: This method does **not** poll Redis every time, because polling
    /// Redis is significantly more time consuming that simply returning the
    /// message already in a queue.  Thus, we only poll Redis if it has not
    /// been polled lately.
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let (timeline, id) = (self.timeline.clone(), self.manager_id);
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

        // Record current time as last polled time
        self.msg_queues.update_time_for_target_queue(id);

        // If the `msg_queue` being polled has any new messages, return the first (oldest) one
        match self.msg_queues.oldest_msg_in_target_queue(id, timeline) {
            Some(value) => Ok(Async::Ready(Some(value))),
            _ => Ok(Async::NotReady),
        }
    }
}
