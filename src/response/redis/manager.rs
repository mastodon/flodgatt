//! Receives data from Redis, sorts it by `ClientAgent`, and stores it until
//! polled by the correct `ClientAgent`.  Also manages sububscriptions and
//! unsubscriptions to/from Redis.
mod err;
pub use err::ManagerErr;

use super::{RedisCmd, RedisConn};
use crate::config;
use crate::event::Event;
use crate::request::{Stream, Subscription, Timeline};

use futures::{Async, Stream as _Stream};
use hashbrown::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};

type Result<T> = std::result::Result<T, ManagerErr>;

/// The item that streams from Redis and is polled by the `ClientAgent`
#[derive(Debug)]
pub struct Manager {
    redis_connection: RedisConn,
    clients_per_timeline: HashMap<Timeline, i32>,
    tx: watch::Sender<(Timeline, Event)>,
    rx: mpsc::UnboundedReceiver<Timeline>,
    ping_time: Instant,
}

impl Manager {
    /// Create a new `Manager`, with its own Redis connections (but, as yet, no
    /// active subscriptions).
    pub fn try_from(
        redis_cfg: config::Redis,
        tx: watch::Sender<(Timeline, Event)>,
        rx: mpsc::UnboundedReceiver<Timeline>,
    ) -> Result<Self> {
        Ok(Self {
            redis_connection: RedisConn::new(redis_cfg)?,
            clients_per_timeline: HashMap::new(),
            tx,
            rx,
            ping_time: Instant::now(),
        })
    }

    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    pub fn subscribe(&mut self, subscription: &Subscription) {
        let (tag, tl) = (subscription.hashtag_name.clone(), subscription.timeline);
        if let (Some(hashtag), Timeline(Stream::Hashtag(id), _, _)) = (tag, tl) {
            self.redis_connection.update_cache(hashtag, id);
        };

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

    pub fn unsubscribe(&mut self, tl: Timeline) -> Result<()> {
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
        while let Ok(Async::Ready(Some(tl))) = self.rx.poll() {
            self.unsubscribe(tl)?
        }

        if self.ping_time.elapsed() > Duration::from_secs(30) {
            self.ping_time = Instant::now();
            self.tx.broadcast((Timeline::empty(), Event::Ping))?
        } else {
            match self.redis_connection.poll_redis() {
                Ok(Async::NotReady) => (),
                Ok(Async::Ready(Some((timeline, event)))) => {
                    self.tx.broadcast((timeline, event))?
                }
                Ok(Async::Ready(None)) => (), // subscription cmd or msg for other namespace
                Err(err) => log::error!("{}", err), // drop msg, log err, and proceed
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
