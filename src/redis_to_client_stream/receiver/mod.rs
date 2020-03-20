//! Receives data from Redis, sorts it by `ClientAgent`, and stores it until
//! polled by the correct `ClientAgent`.  Also manages sububscriptions and
//! unsubscriptions to/from Redis.
mod message_queues;
use crate::{
    config::{self, RedisInterval},
    log_fatal,
    parse_client_request::subscription::{self, postgres, PgPool, Timeline},
    pubsub_cmd,
    redis_to_client_stream::redis::{redis_cmd, RedisConn, RedisStream},
};
use futures::{Async, Poll};
use lru::LruCache;
pub use message_queues::{MessageQueues, MsgQueue};
use serde_json::Value;
use std::{collections::HashMap, net, time};
use tokio::io::Error;
use uuid::Uuid;

/// The item that streams from Redis and is polled by the `ClientAgent`
#[derive(Debug)]
pub struct Receiver {
    pub pubsub_connection: RedisStream,
    secondary_redis_connection: net::TcpStream,
    redis_poll_interval: RedisInterval,
    redis_polled_at: time::Instant,
    timeline: Timeline,
    manager_id: Uuid,
    pub msg_queues: MessageQueues,
    clients_per_timeline: HashMap<Timeline, i32>,
    cache: Cache,
    pool: PgPool,
}
#[derive(Debug)]
struct Cache {
    id_to_hashtag: LruCache<i64, String>,
    hashtag_to_id: LruCache<String, i64>,
}
impl Cache {
    fn new(size: usize) -> Self {
        Self {
            id_to_hashtag: LruCache::new(size),
            hashtag_to_id: LruCache::new(size),
        }
    }
}
impl Receiver {
    /// Create a new `Receiver`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn new(redis_cfg: config::RedisConfig, pool: PgPool) -> Self {
        let RedisConn {
            primary: pubsub_connection,
            secondary: secondary_redis_connection,
            namespace: redis_namespace,
            polling_interval: redis_poll_interval,
        } = RedisConn::new(redis_cfg);

        Self {
            pubsub_connection: RedisStream::from_stream(pubsub_connection)
                .with_namespace(redis_namespace),
            secondary_redis_connection,
            redis_poll_interval,
            redis_polled_at: time::Instant::now(),
            timeline: Timeline::empty(),
            manager_id: Uuid::default(),
            msg_queues: MessageQueues(HashMap::new()),
            clients_per_timeline: HashMap::new(),
            cache: Cache::new(1000), // should this be a run-time option?
            pool,
        }
    }

    /// Assigns the `Receiver` a new timeline to monitor and runs other
    /// first-time setup.
    ///
    /// Note: this method calls `subscribe_or_unsubscribe_as_needed`,
    /// so Redis PubSub subscriptions are only updated when a new timeline
    /// comes under management for the first time.
    pub fn manage_new_timeline(&mut self, manager_id: Uuid, timeline: Timeline) {
        self.manager_id = manager_id;
        self.timeline = timeline;
        self.msg_queues
            .insert(self.manager_id, MsgQueue::new(timeline));
        self.subscribe_or_unsubscribe_as_needed(timeline);
    }

    /// Set the `Receiver`'s manager_id and target_timeline fields to the appropriate
    /// value to be polled by the current `StreamManager`.
    pub fn configure_for_polling(&mut self, manager_id: Uuid, timeline: Timeline) {
        self.manager_id = manager_id;
        self.timeline = timeline;
    }

    fn if_hashtag_timeline_get_hashtag_name(&mut self, timeline: Timeline) -> Option<String> {
        use subscription::Stream::*;
        if let Timeline(Hashtag(id), _, _) = timeline {
            let cached_tag = self.cache.id_to_hashtag.get(&id).map(String::from);
            let tag = match cached_tag {
                Some(tag) => tag,
                None => {
                    let new_tag = postgres::select_hashtag_name(&id, self.pool.clone())
                        .unwrap_or_else(|_| log_fatal!("No hashtag associated with tag #{}", &id));
                    self.cache.hashtag_to_id.put(new_tag.clone(), id);
                    self.cache.id_to_hashtag.put(id, new_tag.clone());
                    new_tag.to_string()
                }
            };
            Some(tag)
        } else {
            None
        }
    }

    /// Drop any PubSub subscriptions that don't have active clients and check
    /// that there's a subscription to the current one.  If there isn't, then
    /// subscribe to it.
    fn subscribe_or_unsubscribe_as_needed(&mut self, timeline: Timeline) {
        let start_time = std::time::Instant::now();
        let timelines_to_modify = self.msg_queues.calculate_timelines_to_add_or_drop(timeline);

        // Record the lower number of clients subscribed to that channel
        for change in timelines_to_modify {
            let timeline = change.timeline;
            let opt_hashtag = self.if_hashtag_timeline_get_hashtag_name(timeline);
            let opt_hashtag = opt_hashtag.as_ref();

            let count_of_subscribed_clients = self
                .clients_per_timeline
                .entry(timeline)
                .and_modify(|n| *n += change.in_subscriber_number)
                .or_insert_with(|| 1);

            // If no clients, unsubscribe from the channel
            if *count_of_subscribed_clients <= 0 {
                pubsub_cmd!("unsubscribe", self, timeline.to_redis_str(opt_hashtag));
            } else if *count_of_subscribed_clients == 1 && change.in_subscriber_number == 1 {
                pubsub_cmd!("subscribe", self, timeline.to_redis_str(opt_hashtag));
            }
        }
        if start_time.elapsed().as_millis() > 1 {
            log::warn!("Sending cmd to Redis took: {:?}", start_time.elapsed());
        };
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
        let (timeline, id) = (self.timeline.clone(), self.manager_id);
        if self.redis_polled_at.elapsed() > *self.redis_poll_interval {
            for (raw_timeline, msg_value) in self.pubsub_connection.poll_redis() {
                let hashtag = if raw_timeline.starts_with("hashtag") {
                    let tag_name = raw_timeline
                        .split(':')
                        .nth(1)
                        .unwrap_or_else(|| log_fatal!("No hashtag found in `{}`", raw_timeline))
                        .to_string();
                    let tag_id = *self
                        .cache
                        .hashtag_to_id
                        .get(&tag_name)
                        .unwrap_or_else(|| log_fatal!("No cached id for `{}`", tag_name));
                    Some(tag_id)
                } else {
                    None
                };
                let timeline = Timeline::from_redis_str(&raw_timeline, hashtag);
                for msg_queue in self.msg_queues.values_mut() {
                    if msg_queue.timeline == timeline {
                        msg_queue.messages.push_back(msg_value.clone());
                    }
                }
            }
            self.redis_polled_at = time::Instant::now();
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
