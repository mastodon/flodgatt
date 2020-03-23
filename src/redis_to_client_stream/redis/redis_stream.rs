use super::redis_msg::RedisMsg;
use crate::config::RedisNamespace;
use crate::log_fatal;
use crate::parse_client_request::subscription::Timeline;
use crate::redis_to_client_stream::receiver::MessageQueues;
use futures::{Async, Poll};
use lru::LruCache;
use std::{io::Read, net};
use tokio::io::AsyncRead;

#[derive(Debug)]
pub struct RedisStream {
    pub inner: net::TcpStream,
    incoming_raw_msg: String,
    pub namespace: RedisNamespace,
}

impl RedisStream {
    pub fn from_stream(inner: net::TcpStream) -> Self {
        RedisStream {
            inner,
            incoming_raw_msg: String::new(),
            namespace: RedisNamespace(None),
        }
    }
    pub fn with_namespace(self, namespace: RedisNamespace) -> Self {
        RedisStream { namespace, ..self }
    }
    // Text comes in from redis as a raw stream, which could be more than one message
    // and is not guaranteed to end on a message boundary.  We need to break it down
    // into messages.  Incoming messages *are* guaranteed to be RESP arrays,
    // https://redis.io/topics/protocol
    /// Adds any new Redis messages to the `MsgQueue` for the appropriate `ClientAgent`.
    pub fn poll_redis(
        &mut self,
        hashtag_to_id_cache: &mut LruCache<String, i64>,
        queues: &mut MessageQueues,
    ) {
        let mut buffer = vec![0u8; 6000];
        if let Ok(Async::Ready(num_bytes_read)) = self.poll_read(&mut buffer) {
            let raw_utf = self.as_utf8(buffer, num_bytes_read);
            self.incoming_raw_msg.push_str(&raw_utf);

            // Only act if we have a full message (end on a msg boundary)
            if !self.incoming_raw_msg.ends_with("}\r\n") {
                return;
            };
            let prefix_to_skip = match &*self.namespace {
                Some(namespace) => format!("{}:timeline:", namespace),
                None => "timeline:".to_string(),
            };

            let mut msg = RedisMsg::from_raw(&self.incoming_raw_msg, prefix_to_skip.len());

            while !msg.raw.is_empty() {
                let command = msg.next_field();
                match command.as_str() {
                    "message" => {
                        let (raw_timeline, msg_value) = msg.extract_raw_timeline_and_message();
                        let hashtag = hashtag_from_timeline(&raw_timeline, hashtag_to_id_cache);
                        let timeline = Timeline::from_redis_str(&raw_timeline, hashtag);
                        for msg_queue in queues.values_mut() {
                            if msg_queue.timeline == timeline {
                                msg_queue.messages.push_back(msg_value.clone());
                            }
                        }
                    }

                    "subscribe" | "unsubscribe" => {
                        // No msg, so ignore & advance cursor to end
                        let _channel = msg.next_field();
                        msg.cursor += ":".len();
                        let _active_subscriptions = msg.process_number();
                        msg.cursor += "\r\n".len();
                    }
                    cmd => panic!("Invariant violation: {} is unexpected Redis output", cmd),
                };
                msg = RedisMsg::from_raw(&msg.raw[msg.cursor..], msg.prefix_len);
            }
            self.incoming_raw_msg.clear();
        }
    }

    fn as_utf8(&mut self, cur_buffer: Vec<u8>, size: usize) -> String {
        String::from_utf8(cur_buffer[..size].to_vec()).unwrap_or_else(|_| {
            let mut new_buffer = vec![0u8; 1];
            self.poll_read(&mut new_buffer).unwrap();
            let buffer = ([cur_buffer, new_buffer]).concat();
            self.as_utf8(buffer, size + 1)
        })
    }
}

fn hashtag_from_timeline(
    raw_timeline: &str,
    hashtag_id_cache: &mut LruCache<String, i64>,
) -> Option<i64> {
    if raw_timeline.starts_with("hashtag") {
        let tag_name = raw_timeline
            .split(':')
            .nth(1)
            .unwrap_or_else(|| log_fatal!("No hashtag found in `{}`", raw_timeline))
            .to_string();

        let tag_id = *hashtag_id_cache
            .get(&tag_name)
            .unwrap_or_else(|| log_fatal!("No cached id for `{}`", tag_name));
        Some(tag_id)
    } else {
        None
    }
}

impl std::ops::Deref for RedisStream {
    type Target = net::TcpStream;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for RedisStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Read for RedisStream {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.inner.read(buffer)
    }
}

impl AsyncRead for RedisStream {
    fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, std::io::Error> {
        match self.read(buf) {
            Ok(t) => Ok(Async::Ready(t)),
            Err(_) => Ok(Async::NotReady),
        }
    }
}
