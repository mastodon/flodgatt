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
    pubsub_cmd,
    redis_to_client_stream::redis::redis_msg::RedisMsg,
    redis_to_client_stream::redis::{redis_cmd, RedisConn},
};
use futures::{Async, Poll};
use lru::LruCache;
use tokio::io::AsyncRead;

use std::{
    collections::HashMap,
    io::Read,
    net, str,
    time::{Duration, Instant},
};
use tokio::io::Error;
use uuid::Uuid;

/// The item that streams from Redis and is polled by the `ClientAgent`
#[derive(Debug)]
pub struct Receiver {
    pub pubsub_connection: net::TcpStream,
    secondary_redis_connection: net::TcpStream,
    redis_poll_interval: Duration,
    redis_polled_at: Instant,
    timeline: Timeline,
    manager_id: Uuid,
    pub msg_queues: MessageQueues,
    clients_per_timeline: HashMap<Timeline, i32>,
    cache: Cache,
    redis_input: Vec<u8>,
    redis_namespace: Option<String>,
}

#[derive(Debug)]
pub struct Cache {
    // TODO: eventually, it might make sense to have Mastodon publish to timelines with
    //       the tag number instead of the tag name.  This would save us from dealing
    //       with a cache here and would be consistent with how lists/users are handled.
    id_to_hashtag: LruCache<i64, String>,
    pub hashtag_to_id: LruCache<String, i64>,
}

impl Receiver {
    /// Create a new `Receiver`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn new(redis_cfg: config::RedisConfig) -> Self {
        let redis_namespace = redis_cfg.namespace.clone();

        let RedisConn {
            primary: pubsub_connection,
            secondary: secondary_redis_connection,
            polling_interval: redis_poll_interval,
        } = RedisConn::new(redis_cfg);

        Self {
            pubsub_connection,
            secondary_redis_connection,
            redis_poll_interval,
            redis_polled_at: Instant::now(),
            timeline: Timeline::empty(),
            manager_id: Uuid::default(),
            msg_queues: MessageQueues(HashMap::new()),
            clients_per_timeline: HashMap::new(),
            cache: Cache {
                id_to_hashtag: LruCache::new(1000),
                hashtag_to_id: LruCache::new(1000),
            }, // should these be run-time options?
            redis_input: Vec::new(),
            redis_namespace,
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
            self.cache.id_to_hashtag.put(id, hashtag.clone());
            self.cache.hashtag_to_id.put(hashtag, id);
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
                Timeline(Stream::Hashtag(id), _, _) => self.cache.id_to_hashtag.get(&id),
                _non_hashtag_timeline => None,
            };

            let count_of_subscribed_clients = self
                .clients_per_timeline
                .entry(timeline)
                .and_modify(|n| *n += change.in_subscriber_number)
                .or_insert_with(|| 1);

            // If no clients, unsubscribe from the channel
            if *count_of_subscribed_clients <= 0 {
                pubsub_cmd!("unsubscribe", self, timeline.to_redis_raw_timeline(hashtag));
            } else if *count_of_subscribed_clients == 1 && change.in_subscriber_number == 1 {
                pubsub_cmd!("subscribe", self, timeline.to_redis_raw_timeline(hashtag));
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
    type Error = Error;

    /// Returns the oldest message in the `ClientAgent`'s queue (if any).
    ///
    /// Note: This method does **not** poll Redis every time, because polling
    /// Redis is significantly more time consuming that simply returning the
    /// message already in a queue.  Thus, we only poll Redis if it has not
    /// been polled lately.
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let (timeline, id) = (self.timeline.clone(), self.manager_id);

        if self.redis_polled_at.elapsed() > self.redis_poll_interval {
            let mut buffer = vec![0u8; 6000];
            if let Ok(Async::Ready(bytes_read)) = self.poll_read(&mut buffer) {
                let binary_input = buffer[..bytes_read].to_vec();
                let (input, extra_bytes) = match str::from_utf8(&binary_input) {
                    Ok(input) => (input, "".as_bytes()),
                    Err(e) => {
                        let (valid, after_valid) = binary_input.split_at(e.valid_up_to());
                        let input = str::from_utf8(valid).expect("Guaranteed by `.valid_up_to`");
                        (input, after_valid)
                    }
                };

                let (cache, namespace) = (&mut self.cache.hashtag_to_id, &self.redis_namespace);

                let remaining_input =
                    process_messages(input, cache, namespace, &mut self.msg_queues);

                self.redis_input.extend_from_slice(remaining_input);
                self.redis_input.extend_from_slice(extra_bytes);
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

impl Read for Receiver {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.pubsub_connection.read(buffer)
    }
}

impl AsyncRead for Receiver {
    fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, std::io::Error> {
        match self.read(buf) {
            Ok(t) => Ok(Async::Ready(t)),
            Err(_) => Ok(Async::NotReady),
        }
    }
}

#[must_use]
pub fn process_messages<'a>(
    input: &'a str,
    mut cache: &mut LruCache<String, i64>,
    namespace: &Option<String>,
    msg_queues: &mut MessageQueues,
) -> &'a [u8] {
    let mut remaining_input = input;
    use RedisMsg::*;
    loop {
        match RedisMsg::from_raw(&mut remaining_input, &mut cache, namespace) {
            Ok((EventMsg(timeline, event), rest)) => {
                for msg_queue in msg_queues.values_mut() {
                    if msg_queue.timeline == timeline {
                        msg_queue.messages.push_back(event.clone());
                    }
                }
                remaining_input = rest;
            }
            Ok((SubscriptionMsg, rest)) | Ok((MsgForDifferentNamespace, rest)) => {
                remaining_input = rest;
            }
            Err(RedisParseErr::Incomplete) => break,
            Err(RedisParseErr::Unrecoverable) => {
                panic!("Failed parsing Redis msg: {}", &remaining_input)
            }
        };
    }
    remaining_input.as_bytes()
}
