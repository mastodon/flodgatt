//! Interfacing with Redis and stream the results on to the `StreamManager`
use crate::redis_cmd;
use crate::user::User;
use futures::stream::Stream;
use futures::{Async, Poll};
use log::info;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, Error};
use uuid::Uuid;

#[derive(Debug)]
struct MsgQueue {
    messages: VecDeque<Value>,
    last_polled_at: Instant,
    redis_channel: String,
}
impl MsgQueue {
    fn new(redis_channel: impl std::fmt::Display) -> Self {
        let redis_channel = redis_channel.to_string();
        MsgQueue {
            messages: VecDeque::new(),
            last_polled_at: Instant::now(),
            redis_channel,
        }
    }
}

/// The item that streams from Redis and is polled by the `StreamManger`
#[derive(Debug)]
pub struct Receiver {
    pubsub_connection: TcpStream,
    secondary_redis_connection: TcpStream,
    tl: String,
    pub user: User,
    manager_id: Uuid,
    msg_queues: HashMap<Uuid, MsgQueue>,
    clients_per_timeline: HashMap<String, i32>,
}
impl Default for Receiver {
    fn default() -> Self {
        Self::new()
    }
}
impl Receiver {
    pub fn new() -> Self {
        let pubsub_connection = TcpStream::connect("127.0.0.1:6379").expect("Can connect to Redis");
        pubsub_connection
            .set_read_timeout(Some(Duration::from_millis(10)))
            .expect("Can set read timeout for Redis connection");
        let secondary_redis_connection =
            TcpStream::connect("127.0.0.1:6379").expect("Can connect to Redis");
        secondary_redis_connection
            .set_read_timeout(Some(Duration::from_millis(10)))
            .expect("Can set read timeout for Redis connection");
        Self {
            pubsub_connection,
            secondary_redis_connection,
            tl: String::new(),
            user: User::public(),
            manager_id: Uuid::new_v4(),
            msg_queues: HashMap::new(),
            clients_per_timeline: HashMap::new(),
        }
    }

    /// Update the `StreamManager` that is currently polling the `Receiver`
    pub fn update(&mut self, id: Uuid, timeline: impl std::fmt::Display) {
        self.manager_id = id;
        self.tl = timeline.to_string();
    }

    /// Send a subscribe command to the Redis PubSub (if needed)
    pub fn maybe_subscribe(&mut self, tl: &str) {
        info!("Subscribing to {}", &tl);

        let manager_id = self.manager_id;
        self.msg_queues.insert(manager_id, MsgQueue::new(tl));
        let current_clients = self
            .clients_per_timeline
            .entry(tl.to_string())
            .and_modify(|n| *n += 1)
            .or_insert(1);

        if *current_clients == 1 {
            let subscribe_cmd = redis_cmd::pubsub("subscribe", tl);
            self.pubsub_connection
                .write_all(&subscribe_cmd)
                .expect("Can subscribe to Redis");
            let set_subscribed_cmd = redis_cmd::set(format!("subscribed:timeline:{}", tl), "1");
            self.secondary_redis_connection
                .write_all(&set_subscribed_cmd)
                .expect("Can set Redis");
            info!("Now subscribed to: {:#?}", &self.msg_queues);
        }
    }

    /// Drop any PubSub subscriptions that don't have active clients
    pub fn unsubscribe_from_empty_channels(&mut self) {
        let mut timelines_with_fewer_clients = Vec::new();

        // Keep only message queues that have been polled recently
        self.msg_queues.retain(|_id, msg_queue| {
            if msg_queue.last_polled_at.elapsed() < Duration::from_secs(30) {
                true
            } else {
                timelines_with_fewer_clients.push(msg_queue.redis_channel.clone());
                false
            }
        });

        // Record the lower number of clients subscribed to that channel
        for timeline in timelines_with_fewer_clients {
            let count_of_subscribed_clients = self
                .clients_per_timeline
                .entry(timeline.clone())
                .and_modify(|n| *n -= 1)
                .or_insert(0);
            // If no clients, unsubscribe from the channel
            if *count_of_subscribed_clients <= 0 {
                self.unsubscribe(&timeline);
            }
        }
    }

    /// Send an unsubscribe command to the Redis PubSub
    pub fn unsubscribe(&mut self, tl: &str) {
        let unsubscribe_cmd = redis_cmd::pubsub("unsubscribe", tl);
        info!("Unsubscribing from {}", &tl);
        self.pubsub_connection
            .write_all(&unsubscribe_cmd)
            .expect("Can unsubscribe from Redis");
        let set_subscribed_cmd = redis_cmd::set(format!("subscribed:timeline:{}", tl), "0");
        self.secondary_redis_connection
            .write_all(&set_subscribed_cmd)
            .expect("Can set Redis");
        info!("Now subscribed only to: {:#?}", &self.msg_queues);
    }
}
impl Stream for Receiver {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let mut buffer = vec![0u8; 3000];
        info!("Being polled by: {}", self.manager_id);
        let timeline = self.tl.clone();

        // Record current time as last polled time
        self.msg_queues
            .entry(self.manager_id)
            .and_modify(|msg_queue| msg_queue.last_polled_at = Instant::now());

        // Add any incomming messages to the back of the relevant `msg_queues`
        // NOTE: This could be more/other than the `msg_queue` currently being polled
        let mut async_stream = AsyncReadableStream(&mut self.pubsub_connection);
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
            .or_insert_with(|| MsgQueue::new(timeline))
            .messages
            .pop_front()
        {
            Some(value) => Ok(Async::Ready(Some(value))),
            _ => Ok(Async::NotReady),
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        let timeline = self.tl.clone();
        self.unsubscribe(&timeline);
    }
}

struct AsyncReadableStream<'a>(&'a mut TcpStream);

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
