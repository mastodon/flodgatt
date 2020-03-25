use super::redis_msg::{ParseErr, RedisMsg};
use crate::config::RedisNamespace;
use crate::log_fatal;
use crate::redis_to_client_stream::receiver::MessageQueues;
use futures::{Async, Poll};
use lru::LruCache;
use std::{error::Error, io::Read, net};
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
    // Text comes in from redis as a raw stream, which could be more than one message and
    // is not guaranteed to end on a message boundary.  We need to break it down into
    // messages.  Incoming messages *are* guaranteed to be RESP arrays (though still not
    // guaranteed to end at an array boundary).  See https://redis.io/topics/protocol
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
            match process_messages(
                self.incoming_raw_msg.clone(),
                &mut self.namespace.0,
                hashtag_to_id_cache,
                queues,
            ) {
                Ok(None) => self.incoming_raw_msg.clear(),
                Ok(Some(msg_fragment)) => self.incoming_raw_msg = msg_fragment,
                Err(e) => {
                    log::error!("{}", e);
                    log_fatal!("Could not process RedisStream: {:?}", &self);
                }
            }
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

type HashtagCache = LruCache<String, i64>;
pub fn process_messages(
    raw_msg: String,
    namespace: &mut Option<String>,
    cache: &mut HashtagCache,
    queues: &mut MessageQueues,
) -> Result<Option<String>, Box<dyn Error>> {
    let prefix_len = match namespace {
        Some(namespace) => format!("{}:timeline:", namespace).len(),
        None => "timeline:".len(),
    };

    let mut input = raw_msg.as_str();
    loop {
        let rest = match RedisMsg::from_raw(&input, cache, prefix_len) {
            Ok((RedisMsg::EventMsg(timeline, event), rest)) => {
                for msg_queue in queues.values_mut() {
                    if msg_queue.timeline == timeline {
                        msg_queue.messages.push_back(event.clone());
                    }
                }
                rest
            }
            Ok((RedisMsg::SubscriptionMsg, rest)) => rest,
            Err(ParseErr::Incomplete) => break,
            Err(ParseErr::Unrecoverable) => log_fatal!("Failed parsing Redis msg: {}", &input),
        };
        input = rest
    }

    Ok(Some(input.to_string()))
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
