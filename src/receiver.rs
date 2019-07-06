//! Interface with Redis and stream the results to the `StreamManager`
//! There is only one `Receiver`, which suggests that it's name is bad.
//!
//! **TODO**: Consider changing the name.  Maybe RedisConnectionPool?
//! There are many AsyncReadableStreams, though.  How do they fit in?
//! Figure this out ASAP.
//! A new one is created every time the Receiver is polled
use crate::{config, pubsub_cmd, redis_cmd};
use futures::{Async, Poll};
use log::info;
use regex::Regex;
use serde_json::Value;
use std::{collections, io::Read, io::Write, net, time};
use tokio::io::{AsyncRead, Error};
use uuid::Uuid;

/// The item that streams from Redis and is polled by the `StreamManager`
#[derive(Debug)]
pub struct Receiver {
    pubsub_connection: net::TcpStream,
    secondary_redis_connection: net::TcpStream,
    tl: String,
    manager_id: Uuid,
    msg_queues: collections::HashMap<Uuid, MsgQueue>,
    clients_per_timeline: collections::HashMap<String, i32>,
}

impl Receiver {
    /// Create a new `Receiver`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn new() -> Self {
        let (pubsub_connection, secondary_redis_connection) = config::redis_addr();
        Self {
            pubsub_connection,
            secondary_redis_connection,
            tl: String::new(),
            manager_id: Uuid::default(),
            msg_queues: collections::HashMap::new(),
            clients_per_timeline: collections::HashMap::new(),
        }
    }

    /// Assigns the `Receiver` a new timeline to monitor and runs other
    /// first-time setup.
    ///
    /// Importantly, this method calls `subscribe_or_unsubscribe_as_needed`,
    /// so Redis PubSub subscriptions are only updated when a new timeline
    /// comes under management for the first time.
    pub fn manage_new_timeline(&mut self, manager_id: Uuid, timeline: &str) {
        self.manager_id = manager_id;
        self.tl = timeline.to_string();
        let old_value = self
            .msg_queues
            .insert(self.manager_id, MsgQueue::new(timeline));
        // Consider removing/refactoring
        if let Some(value) = old_value {
            eprintln!(
                "Data was overwritten when it shouldn't have been.  Old data was: {:#?}",
                value
            );
        }
        self.subscribe_or_unsubscribe_as_needed(timeline);
    }

    /// Set the `Receiver`'s manager_id and target_timeline fields to the approprate
    /// value to be polled by the current `StreamManager`.
    pub fn configure_for_polling(&mut self, manager_id: Uuid, timeline: &str) {
        if &manager_id != &self.manager_id {
            //println!("New Manager: {}", &manager_id);
        }
        self.manager_id = manager_id;
        self.tl = timeline.to_string();
    }

    /// Drop any PubSub subscriptions that don't have active clients and check
    /// that there's a subscription to the current one.  If there isn't, then
    /// subscribe to it.
    fn subscribe_or_unsubscribe_as_needed(&mut self, tl: &str) {
        let mut timelines_to_modify = Vec::new();
        timelines_to_modify.push((tl.to_owned(), 1));

        // Keep only message queues that have been polled recently
        self.msg_queues.retain(|_id, msg_queue| {
            if msg_queue.last_polled_at.elapsed() < time::Duration::from_secs(30) {
                true
            } else {
                let timeline = msg_queue.redis_channel.clone();
                timelines_to_modify.push((timeline, -1));
                false
            }
        });

        // Record the lower number of clients subscribed to that channel
        for (timeline, numerical_change) in timelines_to_modify {
            let mut need_to_subscribe = false;
            let count_of_subscribed_clients = self
                .clients_per_timeline
                .entry(timeline.to_owned())
                .and_modify(|n| *n += numerical_change)
                .or_insert_with(|| {
                    need_to_subscribe = true;
                    1
                });
            // If no clients, unsubscribe from the channel
            if *count_of_subscribed_clients <= 0 {
                info!("Sent unsubscribe command");
                pubsub_cmd!("unsubscribe", self, timeline.clone());
            }
            if need_to_subscribe {
                info!("Sent subscribe command");
                pubsub_cmd!("subscribe", self, timeline.clone());
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
}
impl Default for Receiver {
    fn default() -> Self {
        Receiver::new()
    }
}

impl futures::stream::Stream for Receiver {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let mut buffer = vec![0u8; 3000];
        let timeline = self.tl.clone();

        // Record current time as last polled time
        self.msg_queues
            .entry(self.manager_id)
            .and_modify(|msg_queue| msg_queue.last_polled_at = time::Instant::now());

        // Add any incomming messages to the back of the relevant `msg_queues`
        // NOTE: This could be more/other than the `msg_queue` currently being polled
        let mut async_stream = AsyncReadableStream::new(&mut self.pubsub_connection);
        if let Async::Ready(num_bytes_read) = async_stream.poll_read(&mut buffer)? {
            let raw_redis_response = &String::from_utf8_lossy(&buffer[..num_bytes_read]);
            // capture everything between `{` and `}` as potential JSON
            let json_regex = Regex::new(r"(?P<json>\{.*\})").expect("Hard-coded");
            // capture the timeline so we know which queues to add it to
            let timeline_regex = Regex::new(r"timeline:(?P<timeline>.*?)\r").expect("Hard-codded");
            if let Some(result) = json_regex.captures(raw_redis_response) {
                let timeline =
                    timeline_regex.captures(raw_redis_response).unwrap()["timeline"].to_string();

                let msg: Value = serde_json::from_str(&result["json"].to_string().clone())?;
                for msg_queue in self.msg_queues.values_mut() {
                    if msg_queue.redis_channel == timeline {
                        msg_queue.messages.push_back(msg.clone());
                    }
                }
            }
        }

        // If the `msg_queue` being polled has any new messages, return the first (oldest) one
        match self
            .msg_queues
            .entry(self.manager_id)
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
        pubsub_cmd!("unsubscribe", self, self.tl.clone());
    }
}

#[derive(Debug, Clone)]
struct MsgQueue {
    pub messages: collections::VecDeque<Value>,
    pub last_polled_at: time::Instant,
    pub redis_channel: String,
}

impl MsgQueue {
    pub fn new(redis_channel: impl std::fmt::Display) -> Self {
        let redis_channel = redis_channel.to_string();
        MsgQueue {
            messages: collections::VecDeque::new(),
            last_polled_at: time::Instant::now(),
            redis_channel,
        }
    }
}

struct AsyncReadableStream<'a>(&'a mut net::TcpStream);
impl<'a> AsyncReadableStream<'a> {
    pub fn new(stream: &'a mut net::TcpStream) -> Self {
        AsyncReadableStream(stream)
    }
}

impl<'a> Read for AsyncReadableStream<'a> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.0.read(buffer)
    }
}

impl<'a> AsyncRead for AsyncReadableStream<'a> {
    fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, std::io::Error> {
        match self.read(buf) {
            Ok(t) => Ok(Async::Ready(t)),
            Err(_) => Ok(Async::NotReady),
        }
    }
}
