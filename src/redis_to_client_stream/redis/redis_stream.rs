use super::redis_msg::RedisMsg;
use crate::config::RedisNamespace;
use futures::{Async, Poll};
use serde_json::Value;
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
    pub fn poll_redis(&mut self) -> Vec<(String, Value)> {
        let mut buffer = vec![0u8; 6000];
        let mut messages = Vec::new();

        if let Async::Ready(num_bytes_read) = self.poll_read(&mut buffer).unwrap() {
            let raw_utf = self.as_utf8(buffer, num_bytes_read);
            self.incoming_raw_msg.push_str(&raw_utf);

            // Only act if we have a full message (end on a msg boundary)
            if !self.incoming_raw_msg.ends_with("}\r\n") {
                return messages;
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
                        messages.push((raw_timeline, msg_value));
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
        messages
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
