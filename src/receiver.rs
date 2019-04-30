//! Interfacing with Redis and stream the results on to the `StreamManager`
use crate::user::User;
use futures::stream::Stream;
use futures::{Async, Poll};
use log::info;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use tokio::io::{AsyncRead, Error};
use uuid::Uuid;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// The item that streams from Redis and is polled by the `StreamManger`
#[derive(Debug)]
pub struct Receiver {
    stream: TcpStream,
    tl: String,
    pub user: User,
    polled_by: Uuid,
    msg_queue: HashMap<Uuid, VecDeque<Value>>,
}
impl Receiver {
    pub fn new() -> Self {
        let stream = TcpStream::connect("127.0.0.1:6379").unwrap();

        stream
            .set_read_timeout(Some(Duration::from_millis(10)))
            .unwrap();
        Self {
            stream,
            tl: String::new(),
            user: User::public(),
            polled_by: Uuid::new_v4(),
            msg_queue: HashMap::new(),
        }
    }
    /// Update the `StreamManager` that is currently polling the `Receiver`
    pub fn set_polled_by(&mut self, id: Uuid) -> &Self {
        self.polled_by = id;
        self
    }
    /// Send a subscribe command to the Redis PubSub
    pub fn subscribe(&mut self, tl: &str) {
        let subscribe_cmd = redis_cmd_from("subscribe", &tl);
        info!("Subscribing to {}", &tl);
        self.stream
            .write_all(&subscribe_cmd)
            .expect("Can subscribe to Redis");
    }
    /// Send an unsubscribe command to the Redis PubSub
    pub fn unsubscribe(&mut self, tl: &str) {
        let unsubscribe_cmd = redis_cmd_from("unsubscribe", &tl);
        info!("Subscribing to {}", &tl);
        self.stream
            .write_all(&unsubscribe_cmd)
            .expect("Can unsubscribe from Redis");
    }
}
impl Stream for Receiver {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let mut buffer = vec![0u8; 3000];
        let polled_by = self.polled_by;
        self.msg_queue
            .entry(polled_by)
            .or_insert_with(VecDeque::new);
        info!("Being polled by StreamManager with uuid: {}", polled_by);

        let mut async_stream = AsyncReadableStream(&mut self.stream);

        if let Async::Ready(num_bytes_read) = async_stream.poll_read(&mut buffer)? {
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
        let timeline = self.tl.clone();
        self.unsubscribe(&timeline);
    }
}

struct AsyncReadableStream<'a>(&'a mut TcpStream);

impl<'a> Read for AsyncReadableStream<'a> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.0.read(buffer)
    }
}

impl<'a> AsyncRead for AsyncReadableStream<'a> {
    fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, std::io::Error> {
        match self.read(buf) {
            Ok(t) => Ok(Async::Ready(t)),
            Err(_) => Ok(Async::NotReady),
        }
    }
}

fn redis_cmd_from(cmd: impl std::fmt::Display, timeline: impl std::fmt::Display) -> Vec<u8> {
    let (cmd, arg) = (cmd.to_string(), format!("timeline:{}", timeline));
    format!(
        "*2\r\n${cmd_length}\r\n{cmd}\r\n${arg_length}\r\n{arg}\r\n",
        cmd_length = cmd.len(),
        cmd = cmd,
        arg_length = arg.len(),
        arg = arg
    )
    .as_bytes()
    .to_owned()
}
