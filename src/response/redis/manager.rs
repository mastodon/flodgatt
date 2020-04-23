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

use futures::{Async, Stream as _Stream};
use hashbrown::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};

type Result<T> = std::result::Result<T, Error>;

/// The item that streams from Redis and is polled by the `ClientAgent`
#[derive(Debug)]
pub struct Manager {
    redis_connection: RedisConn,
    clients_per_timeline: HashMap<Timeline, i32>,
    tx: watch::Sender<(Timeline, Event)>,
    timelines: HashMap<Timeline, Vec<mpsc::UnboundedSender<Event>>>,
    rx: mpsc::UnboundedReceiver<Timeline>,
    ping_time: Instant,
}

impl Manager {
    /// Create a new `Manager`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn try_from(
        redis_cfg: &config::Redis,
        tx: watch::Sender<(Timeline, Event)>,
        rx: mpsc::UnboundedReceiver<Timeline>,
    ) -> Result<Self> {
        Ok(Self {
            redis_connection: RedisConn::new(redis_cfg)?,
            clients_per_timeline: HashMap::new(),
            timelines: HashMap::new(),
            tx,
            rx,
            ping_time: Instant::now(),
        })
    }

    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    pub fn subscribe(
        &mut self,
        subscription: &Subscription,
        channel: mpsc::UnboundedSender<Event>,
    ) {
        let (tag, tl) = (subscription.hashtag_name.clone(), subscription.timeline);
        if let (Some(hashtag), Some(id)) = (tag, tl.tag()) {
            self.redis_connection.update_cache(hashtag, id);
        };

        self.timelines
            .entry(tl)
            .and_modify(|vec| vec.push(channel.clone()))
            .or_insert_with(|| vec![channel]);

        let number_of_subscriptions = self
            .clients_per_timeline
            .entry(tl)
            .and_modify(|n| *n += 1)
            .or_insert(1);

        use RedisCmd::*;
        if *number_of_subscriptions == 1 {
            self.redis_connection
                .send_cmd(Subscribe, &tl)
                .unwrap_or_else(|e| log::error!("Could not subscribe to the Redis channel: {}", e));
        };
    }

    pub(crate) fn unsubscribe(
        &mut self,
        tl: Timeline,
        _target_channel: mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        let channels = self.timelines.get(&tl).expect("TODO");
        for (_i, _channel) in channels.iter().enumerate() {
            // TODO - find alternate implementation
        }

        let number_of_subscriptions = self
            .clients_per_timeline
            .entry(tl)
            .and_modify(|n| *n -= 1)
            .or_insert_with(|| {
                log::error!(
                    "Attempted to unsubscribe from a timeline to which you were not subscribed: {:?}",
                    tl
                );
                0
            });
        use RedisCmd::*;
        if *number_of_subscriptions == 0 {
            self.redis_connection.send_cmd(Unsubscribe, &tl)?;
            self.clients_per_timeline.remove_entry(&tl);
        };
        log::info!("Ended stream for {:?}", tl);
        Ok(())
    }

    pub fn poll_broadcast(&mut self) -> Result<()> {
        // while let Ok(Async::Ready(Some(tl))) = self.rx.poll() {
        //     self.unsubscribe(tl)?
        // }
        let mut completed_timelines = Vec::new();
        if self.ping_time.elapsed() > Duration::from_secs(30) {
            self.ping_time = Instant::now();
            for (timeline, channels) in self.timelines.iter_mut() {
                for channel in channels.iter_mut() {
                    match channel.try_send(Event::Ping) {
                        Ok(_) => (),
                        Err(_) => completed_timelines.push((*timeline, channel.clone())),
                    }
                }
            }
        };
        loop {
            match self.redis_connection.poll_redis() {
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(Some((timeline, event)))) => {
                    for channel in self.timelines.get_mut(&timeline).ok_or(Error::InvalidId)? {
                        match channel.try_send(event.clone()) {
                            Ok(_) => (),
                            Err(_) => completed_timelines.push((timeline, channel.clone())),
                        }
                    }
                }
                Ok(Async::Ready(None)) => (), // None = cmd or msg for other namespace
                Err(err) => log::error!("{}", err), // drop msg, log err, and proceed
            }
        }

        for (tl, channel) in completed_timelines {
            self.unsubscribe(tl, channel)?;
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
            self.clients_per_timeline.values().sum::<i32>()
        )
    }

    pub fn list(&self) -> String {
        let max_len = self
            .clients_per_timeline
            .keys()
            .fold(0, |acc, el| acc.max(format!("{:?}:", el).len()));
        self.clients_per_timeline
            .iter()
            .map(|(tl, n)| {
                let tl_txt = format!("{:?}:", tl);
                format!("{:>1$} {2}\n", tl_txt, max_len, n)
            })
            .collect()
    }
}
