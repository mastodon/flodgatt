use crate::pubsub;
use crate::pubsub::PubSub;
use crate::user::User;
use futures::stream::Stream;
use futures::{Async, Poll};
use log::info;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncWrite, Error, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use uuid::Uuid;

#[derive(Clone)]
pub struct StreamManager {
    recv: Arc<Mutex<HashMap<String, Receiver>>>,
    last_polled: Arc<Mutex<HashMap<String, Instant>>>,
    current_stream: String,
    id: uuid::Uuid,
}
impl StreamManager {
    pub fn new() -> Self {
        StreamManager {
            recv: Arc::new(Mutex::new(HashMap::new())),
            last_polled: Arc::new(Mutex::new(HashMap::new())),
            current_stream: String::new(),
            id: Uuid::new_v4(),
        }
    }

    pub fn new_copy(&self) -> Self {
        let id = Uuid::new_v4();
        StreamManager { id, ..self.clone() }
    }

    pub fn add(&mut self, timeline: &String, user: &User) -> &Self {
        let mut streams = self.recv.lock().expect("No other thread panic");
        streams
            .entry(timeline.clone())
            .or_insert_with(|| PubSub::from(&timeline, &user));
        let mut last_polled = self.last_polled.lock().expect("No other thread panic");
        last_polled.insert(timeline.clone(), Instant::now());

        // Drop any streams that haven't been polled in the last 30 seconds
        last_polled
            .clone()
            .iter()
            .filter(|(_, time)| time.elapsed().as_secs() > 30)
            .for_each(|(key, _)| {
                last_polled.remove(key);
                streams.remove(key);
            });

        self.current_stream = timeline.clone();
        self
    }
}
impl Stream for StreamManager {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut last_polled = self.last_polled.lock().expect("No other thread panic");
        let target_stream = self.current_stream.clone();
        last_polled.insert(target_stream.clone(), Instant::now());

        let mut streams = self.recv.lock().expect("No other thread panic");
        let shared_conn = streams.get_mut(&target_stream).expect("known key");
        shared_conn.set_polled_by(self.id);

        match shared_conn.poll() {
            Ok(Async::Ready(Some(value))) => Ok(Async::Ready(Some(value))),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug)]
pub struct Receiver {
    rx: ReadHalf<TcpStream>,
    tx: WriteHalf<TcpStream>,
    tl: String,
    pub user: User,
    polled_by: Uuid,
    msg_queue: HashMap<Uuid, VecDeque<Value>>,
}
impl Receiver {
    pub fn new(socket: TcpStream, tl: String, user: &User) -> Self {
        let (rx, mut tx) = socket.split();
        tx.poll_write(pubsub::RedisCmd::subscribe_to_timeline(&tl).as_bytes())
            .expect("Can subscribe to Redis");
        Self {
            rx,
            tx,
            tl,
            user: user.clone(),
            polled_by: Uuid::new_v4(),
            msg_queue: HashMap::new(),
        }
    }
    pub fn set_polled_by(&mut self, id: Uuid) -> &Self {
        self.polled_by = id;
        self
    }
}
impl Stream for Receiver {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let mut buffer = vec![0u8; 3000];
        let polled_by = self.polled_by;
        self.msg_queue.entry(polled_by).or_insert(VecDeque::new());
        info!("Being polled by StreamManager with uuid: {}", polled_by);
        if let Async::Ready(num_bytes_read) = self.rx.poll_read(&mut buffer)? {
            // capture everything between `{` and `}` as potential JSON
            // TODO: figure out if `(?x)` is needed
            let re = Regex::new(r"(?P<json>\{.*\})").expect("Valid hard-coded regex");

            if let Some(cap) = re.captures(&String::from_utf8_lossy(&buffer[..num_bytes_read])) {
                let json: Value = serde_json::from_str(&cap["json"].to_string().clone())?;
                for value in self.msg_queue.values_mut() {
                    value.push_back(json.clone());
                }
            }
        }
        if let Some(value) = self.msg_queue.entry(polled_by).or_default().pop_front() {
            Ok(Async::Ready(Some(value)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let channel = format!("timeline:{}", self.tl);
        self.tx
            .poll_write(pubsub::RedisCmd::unsubscribe_from_timeline(&channel).as_bytes())
            .expect("Can unsubscribe from Redis");
        let open_connections = pubsub::OPEN_CONNECTIONS.fetch_sub(1, Ordering::Relaxed) - 1;
        info!("Receiver dropped.  {} connection(s) open", open_connections);
    }
}
