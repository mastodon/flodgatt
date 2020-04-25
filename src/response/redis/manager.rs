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

use futures::Async;
use hashbrown::{HashMap, HashSet};
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
}

impl Manager {
    /// Create a new `Manager`, with its own Redis connections (but no active subscriptions).
    pub fn try_from(redis_cfg: &config::Redis) -> Result<Self> {
        Ok(Self {
            redis_conn: RedisConn::new(redis_cfg)?,
            timelines: HashMap::new(),
            ping_time: Instant::now(),
            channel_id: 0,
        })
    }

    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    pub fn subscribe(&mut self, subscription: &Subscription, channel: EventChannel) {
        let (tag, tl) = (subscription.hashtag_name.clone(), subscription.timeline);
        if let (Some(hashtag), Some(id)) = (tag, tl.tag()) {
            self.redis_conn.update_cache(hashtag, id);
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
            channels.retain(|_, chan| try_send_event(Arc::new(Event::Ping), chan, *tl).is_ok());

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

    pub fn poll_broadcast(&mut self) -> Result<()> {
        if self.ping_time.elapsed() > Duration::from_secs(30) {
            self.send_pings()?
        }

        let (mut unread_start, mut msg_end) = (0, 0);

        while let Async::Ready(msg_len) = self.redis_conn.poll_redis(msg_end)? {
            msg_end += msg_len;
            let input = &self.redis_conn.input[..msg_end];
            let mut unread = str::from_utf8(input).unwrap_or_else(|e| {
                str::from_utf8(input.split_at(e.valid_up_to()).0).expect("guaranteed by `split_at`")
            });

            while !unread.is_empty() {
                let tag_id_cache = &mut self.redis_conn.tag_id_cache;
                let redis_namespace = &self.redis_conn.namespace;

                use {Error::InvalidId, RedisParseOutput::*};
                unread = match RedisParseOutput::try_from(unread) {
                    Ok(Msg(msg)) => {
                        let trimmed_tl = match redis_namespace {
                            Some(ns) if msg.timeline_txt.starts_with(ns) => {
                                Some(&msg.timeline_txt[ns.len() + ":timeline:".len()..])
                            }
                            None => Some(&msg.timeline_txt["timeline:".len()..]),
                            Some(_non_matching_ns) => None,
                        };

                        if let Some(trimmed_tl) = trimmed_tl {
                            let tl = Timeline::from_redis_text(trimmed_tl, tag_id_cache)?;
                            let event: Arc<Event> = Arc::new(msg.event_txt.try_into()?);
                            let channels = self.timelines.get_mut(&tl).ok_or(InvalidId)?;
                            channels.retain(|_, c| try_send_event(event.clone(), c, tl).is_ok());
                        } else {
                            // skip messages for different Redis namespaces
                        }
                        msg.leftover_input
                    }
                    Ok(NonMsg(leftover_input)) => leftover_input,
                    Err(RedisParseErr::Incomplete) => break,
                    Err(e) => Err(e)?,
                };
                unread_start = msg_end - unread.len();
            }
            if !unread.is_empty() && unread_start > unread.len() {
                log::info!("Re-using memory");
                let (read, unread) = self.redis_conn.input[..msg_end].split_at_mut(unread_start);

                for (i, b) in unread.iter().enumerate() {
                    read[i] = *b;
                }
                msg_end = unread.len();
                unread_start = 0;
            }
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

fn try_send_event(event: Arc<Event>, chan: &mut EventChannel, tl: Timeline) -> Result<()> {
    match chan.try_send(event) {
        Ok(()) => Ok(()),
        Err(e) if !e.is_closed() => {
            log::error!("cannot send to {:?}: {}", tl, e);
            Ok(())
        }
        Err(e) => Err(e)?,
    }
}
