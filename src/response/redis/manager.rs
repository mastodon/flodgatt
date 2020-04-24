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
use hashbrown::{HashMap, HashSet};
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
    /// Create a new `Manager`, with its own Redis connections (but no active subscriptions).
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

    pub(crate) fn unsubscribe(&mut self, tl: &Timeline) -> Result<()> {
        self.redis_connection.send_cmd(RedisCmd::Unsubscribe, &tl)?;
        self.timelines.remove(&tl);
        Ok(log::info!("Ended stream for {:?}", tl))
    }

    pub fn poll_broadcast(&mut self) -> Result<()> {
        let mut completed_timelines = HashSet::new();
        let log_send_err = |tl, e| Some(log::error!("cannot send to {:?}: {}", tl, e)).is_some();

        if self.ping_time.elapsed() > Duration::from_secs(30) {
            self.ping_time = Instant::now();
            for (tl, channels) in self.timelines.iter_mut() {
                channels.retain(|_, chan| match chan.try_send(Arc::new(Event::Ping)) {
                    Ok(()) => true,
                    Err(e) if !e.is_closed() => log_send_err(*tl, e),
                    Err(_) => false,
                });

                // NOTE: this takes two cycles to close a connection after the client
                // times out: on the first cycle, this fn sends the Event to the
                // response::Ws thread without any error, but that thread encounters an
                // error sending to the client and ends.  On the *second* cycle, this fn
                // gets the error it's waiting on to clean up the connection.  This isn't
                // ideal, but is harmless, since the only reason we haven't cleaned up the
                // connection is that no messages are being sent to that client.
                if channels.is_empty() {
                    completed_timelines.insert(*tl);
                }
            }
        };

        loop {
            match self.redis_connection.poll_redis() {
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(Some((tl, event)))) => {
                    let sendable_event = Arc::new(event);
                    let channels = self.timelines.get_mut(&tl).ok_or(Error::InvalidId)?;
                    channels.retain(|_, chan| match chan.try_send(sendable_event.clone()) {
                        Ok(()) => true,
                        Err(e) if !e.is_closed() => log_send_err(tl, e),
                        Err(_) => false,
                    });
                    if channels.is_empty() {
                        completed_timelines.insert(tl);
                    }
                }
                Ok(Async::Ready(None)) => (), // cmd or msg for other namespace
                Err(err) => log::error!("{}", err), // drop msg, log err, and proceed
            }
        }

        for tl in &mut completed_timelines.iter() {
            self.unsubscribe(tl)?;
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
