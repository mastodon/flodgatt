//! Receives data from Redis, sorts it by `ClientAgent`, and stores it until
//! polled by the correct `ClientAgent`.  Also manages sububscriptions and
//! unsubscriptions to/from Redis.
mod err;
pub use err::Error;

use super::msg::{RedisParseErr, RedisParseOutput};
use super::{Event, RedisCmd, RedisConn};
use crate::config;
use crate::request::{Subscription, Timeline};

pub(self) use super::EventErr;

use futures::{Async, Poll, Stream};
use hashbrown::{HashMap, HashSet};
use lru::LruCache;
use std::convert::{TryFrom, TryInto};
use std::str;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;

type Result<T> = std::result::Result<T, Error>;
type EventChannel = Sender<Arc<Event>>;

/// The item that streams from Redis and is polled by the `ClientAgent`
pub struct Manager {
    redis_conn: RedisConn,
    timelines: HashMap<Timeline, HashMap<u32, EventChannel>>,
    ping_time: Instant,
    channel_id: u32,
    unread_idx: (usize, usize),
    tag_id_cache: LruCache<String, i64>,
}

impl Stream for Manager {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<()>, Error> {
        if self.ping_time.elapsed() > Duration::from_secs(30) {
            self.send_pings()?
        }

        while let Async::Ready(msg_len) = self.redis_conn.poll_redis(self.unread_idx.1)? {
            self.unread_idx = (0, self.unread_idx.1 + msg_len);

            let input = &self.redis_conn.input[..self.unread_idx.1];
            let mut unread = str::from_utf8(input).unwrap_or_else(|e| {
                str::from_utf8(input.split_at(e.valid_up_to()).0).expect("guaranteed by `split_at`")
            });

            while !unread.is_empty() {
                use RedisParseOutput::*;
                match RedisParseOutput::try_from(unread) {
                    Ok(Msg(msg)) => {
                        // If we get a message and it matches the redis_namespace, get the msg's
                        // Event and send it to all channels matching the msg's Timeline
                        if let Some(tl) = msg.timeline_matching_ns(&self.redis_conn.namespace) {
                            let tl = Timeline::from_redis_text(tl, &mut self.tag_id_cache)?;
                            let event: Arc<Event> = Arc::new(msg.event_txt.try_into()?);
                            if let Some(channels) = self.timelines.get_mut(&tl) {
                                for channel in channels.values_mut() {
                                    if let Ok(Async::NotReady) = channel.poll_ready() {
                                        log::warn!("{:?} channel full\ncan't send:{:?}", tl, event);
                                        return Ok(Async::NotReady);
                                    }
                                    let _ = channel.try_send(event.clone()); // err just means channel will be closed
                                }
                            }
                        }
                        unread = msg.leftover_input;
                    }
                    Ok(NonMsg(leftover_input)) => unread = leftover_input,
                    Err(RedisParseErr::Incomplete) => {
                        self.copy_partial_msg();
                        unread = "";
                    }
                    Err(e) => Err(Error::RedisParseErr(e, unread.to_string()))?,
                };
                self.unread_idx.0 = self.unread_idx.1 - unread.len();
            }
            if self.unread_idx.0 == self.unread_idx.1 {
                self.unread_idx = (0, 0)
            }
        }
        Ok(Async::Ready(Some(())))
    }
}

impl Manager {
    fn copy_partial_msg(&mut self) {
        if self.unread_idx.0 == 0 {
            // msg already first; no copying needed
        } else if self.unread_idx.0 >= (self.unread_idx.1 - self.unread_idx.0) {
            let (read, unread) =
                self.redis_conn.input[..self.unread_idx.1].split_at_mut(self.unread_idx.0);
            for (i, b) in unread.iter().enumerate() {
                read[i] = *b;
            }
        } else {
            // Less efficient, but should never occur in production
            log::warn!("Moving partial input requires heap allocation");
            self.redis_conn.input = self.redis_conn.input[self.unread_idx.0..].into();
        }
        self.unread_idx = (0, self.unread_idx.1 - self.unread_idx.0);
    }
    /// Create a new `Manager`, with its own Redis connections (but no active subscriptions).
    pub fn try_from(redis_cfg: &config::Redis) -> Result<Self> {
        Ok(Self {
            redis_conn: RedisConn::new(redis_cfg)?,
            timelines: HashMap::new(),
            ping_time: Instant::now(),
            channel_id: 0,
            unread_idx: (0, 0),
            tag_id_cache: LruCache::new(1000),
        })
    }

    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    pub fn subscribe(&mut self, subscription: &Subscription, channel: EventChannel) {
        let (tag, tl) = (subscription.hashtag_name.clone(), subscription.timeline);
        if let (Some(hashtag), Some(id)) = (tag, tl.tag()) {
            self.tag_id_cache.put(hashtag.clone(), id);
            self.redis_conn.tag_name_cache.put(id, hashtag);
        };

        let channels = self.timelines.entry(tl).or_default();
        channels.insert(self.channel_id, channel);
        self.channel_id += 1;

        if channels.len() == 1 {
            self.redis_conn
                .send_cmd(RedisCmd::Subscribe, &[tl])
                .unwrap_or_else(|e| log::error!("Could not subscribe to the Redis channel: {}", e));
            log::info!("Subscribed to {:?}", tl);
        };
    }

    fn send_pings(&mut self) -> Result<()> {
        // NOTE: this takes two cycles to close a connection after the client times out: on
        // the first cycle, this successfully sends the Event to the response::Ws thread but
        // that thread fatally errors sending to the client.  On the *second* cycle, this
        // gets the error.  This isn't ideal, but is harmless.

        self.ping_time = Instant::now();
        let mut subscriptions_to_close = HashSet::new();
        self.timelines.retain(|tl, channels| {
            channels.retain(|_, chan| chan.try_send(Arc::new(Event::Ping)).is_ok());

            if channels.is_empty() {
                subscriptions_to_close.insert(*tl);
                false
            } else {
                true
            }
        });
        if !subscriptions_to_close.is_empty() {
            let timelines: Vec<_> = subscriptions_to_close.into_iter().collect();
            &self
                .redis_conn
                .send_cmd(RedisCmd::Unsubscribe, &timelines[..])?;
            log::info!("Unsubscribed from {:?}", timelines);
        }
        Ok(())
    }

    pub fn recover(poisoned: PoisonError<MutexGuard<Self>>) -> MutexGuard<Self> {
        log::error!("{}", &poisoned);
        poisoned.into_inner()
    }

    pub fn count(&self) -> String {
        format!(
            "Current connections: {}",
            self.timelines.values().map(HashMap::len).sum::<usize>()
        )
    }

    pub fn backpresure(&self) -> String {
        format!(
            "Input buffer size: {} KiB",
            (self.unread_idx.1 - self.unread_idx.0) / 1024
        )
    }

    pub fn list(&self) -> String {
        let max_len = self
            .timelines
            .keys()
            .fold(0, |acc, el| acc.max(format!("{:?}:", el).len()));
        self.timelines
            .iter()
            .map(|(tl, channel_map)| {
                let tl_txt = format!("{:?}:", tl);
                format!("{:>1$} {2}\n", tl_txt, max_len, channel_map.len())
            })
            .chain(std::iter::once(
                "\n*may include recently disconnected clients".to_string(),
            ))
            .collect()
    }
}
