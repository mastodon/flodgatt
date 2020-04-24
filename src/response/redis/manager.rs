//! Receives data from Redis, sorts it by `ClientAgent`, and stores it until
//! polled by the correct `ClientAgent`.  Also manages sububscriptions and
//! unsubscriptions to/from Redis.
mod err;
pub use err::Error;

use super::Event;
use super::{RedisCmd, RedisConn};
use crate::config;
use crate::request::{Subscription, Timeline};

pub(self) use super::EventErr;

use futures::Async;
use hashbrown::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;

type Result<T> = std::result::Result<T, Error>;

/// The item that streams from Redis and is polled by the `ClientAgent`
pub struct Manager {
    redis_connection: RedisConn,
    timelines: HashMap<Timeline, HashMap<u32, Sender<Arc<Event>>>>,
    ping_time: Instant,
    channel_id: u32,
}

impl Manager {
    /// Create a new `Manager`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn try_from(redis_cfg: &config::Redis) -> Result<Self> {
        Ok(Self {
            redis_connection: RedisConn::new(redis_cfg)?,
            timelines: HashMap::new(),
            ping_time: Instant::now(),
            channel_id: 0,
        })
    }

    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    pub fn subscribe(&mut self, subscription: &Subscription, channel: Sender<Arc<Event>>) {
        let (tag, tl) = (subscription.hashtag_name.clone(), subscription.timeline);
        if let (Some(hashtag), Some(id)) = (tag, tl.tag()) {
            self.redis_connection.update_cache(hashtag, id);
        };

        let channels = self.timelines.entry(tl).or_default();
        channels.insert(self.channel_id, channel);
        self.channel_id += 1;

        if channels.len() == 1 {
            self.redis_connection
                .send_cmd(RedisCmd::Subscribe, &tl)
                .unwrap_or_else(|e| log::error!("Could not subscribe to the Redis channel: {}", e));
        };
    }

    pub(crate) fn unsubscribe(&mut self, tl: &mut Timeline, id: &u32) -> Result<()> {
        let channels = self.timelines.get_mut(tl).ok_or(Error::InvalidId)?;
        channels.remove(id);

        if channels.len() == 0 {
            self.redis_connection.send_cmd(RedisCmd::Unsubscribe, &tl)?;
            self.timelines.remove(&tl);
        };
        log::info!("Ended stream for {:?}", tl);
        Ok(())
    }

    pub fn poll_broadcast(&mut self) -> Result<()> {
        let mut completed_timelines = Vec::new();
        if self.ping_time.elapsed() > Duration::from_secs(30) {
            self.ping_time = Instant::now();
            for (timeline, channels) in self.timelines.iter_mut() {
                for (id, channel) in channels.iter_mut() {
                    match channel.try_send(Arc::new(Event::Ping)) {
                        Ok(_) => (),
                        Err(_) => completed_timelines.push((*timeline, *id)),
                    }
                }
            }
        };

        loop {
            match self.redis_connection.poll_redis() {
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(Some((tl, event)))) => {
                    let sendable_event = Arc::new(event);
                    for (uuid, tx) in self.timelines.get_mut(&tl).ok_or(Error::InvalidId)? {
                        tx.try_send(sendable_event.clone())
                            .unwrap_or_else(|_| completed_timelines.push((tl, *uuid)))
                    }
                }
                Ok(Async::Ready(None)) => (), // cmd or msg for other namespace
                Err(err) => log::error!("{}", err), // drop msg, log err, and proceed
            }
        }

        for (tl, channel) in completed_timelines.iter_mut() {
            self.unsubscribe(tl, &channel)?;
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
            self.timelines.values().map(|el| el.len()).sum::<usize>()
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
            .collect()
    }
}
