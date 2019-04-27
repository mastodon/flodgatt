use crate::user::User;
use futures::{Async, Future, Poll};
use log::{debug, info};
use regex::Regex;
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{thread, time};
use tokio::io::{AsyncRead, AsyncWrite, Error, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use warp::Stream;

static OPEN_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
static MAX_CONNECTIONS: AtomicUsize = AtomicUsize::new(400);

struct RedisCmd {
    resp_cmd: String,
}
impl RedisCmd {
    fn new(cmd: impl std::fmt::Display, arg: impl std::fmt::Display) -> Self {
        let (cmd, arg) = (cmd.to_string(), arg.to_string());
        let resp_cmd = format!(
            "*2\r\n${cmd_length}\r\n{cmd}\r\n${arg_length}\r\n{arg}\r\n",
            cmd_length = cmd.len(),
            cmd = cmd,
            arg_length = arg.len(),
            arg = arg
        );
        Self { resp_cmd }
    }
    fn subscribe_to_timeline(timeline: &str) -> String {
        let channel = format!("timeline:{}", timeline);
        let subscribe = RedisCmd::new("subscribe", &channel);
        info!("Subscribing to {}", &channel);
        subscribe.resp_cmd
    }
    fn unsubscribe_from_timeline(timeline: &str) -> String {
        let channel = format!("timeline:{}", timeline);
        let unsubscribe = RedisCmd::new("unsubscribe", &channel);
        info!("Unsubscribing from {}", &channel);
        unsubscribe.resp_cmd
    }
}

#[derive(Debug)]
pub struct Receiver {
    rx: ReadHalf<TcpStream>,
    tx: WriteHalf<TcpStream>,
    tl: String,
    pub user: User,
}
impl Receiver {
    fn new(socket: TcpStream, tl: String, user: User) -> Self {
        println!("created a new Receiver");
        let (rx, mut tx) = socket.split();
        tx.poll_write(RedisCmd::subscribe_to_timeline(&tl).as_bytes())
            .expect("Can subscribe to Redis");
        Self { rx, tx, tl, user }
    }
}
impl Stream for Receiver {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let mut buffer = vec![0u8; 3000];
        if let Async::Ready(num_bytes_read) = self.rx.poll_read(&mut buffer)? {
            // capture everything between `{` and `}` as potential JSON
            let re = Regex::new(r"(?P<json>\{.*\})").expect("Valid hard-coded regex");

            if let Some(cap) = re.captures(&String::from_utf8_lossy(&buffer[..num_bytes_read])) {
                debug!("{}", &cap["json"]);
                let json: Value = serde_json::from_str(&cap["json"].to_string().clone())?;
                return Ok(Async::Ready(Some(json)));
            }
            return Ok(Async::NotReady);
        }
        Ok(Async::NotReady)
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let channel = format!("timeline:{}", self.tl);
        self.tx
            .poll_write(RedisCmd::unsubscribe_from_timeline(&channel).as_bytes())
            .expect("Can unsubscribe from Redis");
        let open_connections = OPEN_CONNECTIONS.fetch_sub(1, Ordering::Relaxed) - 1;
        info!("Receiver dropped.  {} connection(s) open", open_connections);
    }
}

use futures::sink::Sink;
use tokio::net::tcp::ConnectFuture;
struct Socket {
    connect: ConnectFuture,
    tx: tokio::sync::mpsc::Sender<TcpStream>,
}
impl Socket {
    fn new(address: impl std::fmt::Display, tx: tokio::sync::mpsc::Sender<TcpStream>) -> Self {
        let address = address
            .to_string()
            .parse()
            .expect("Unable to parse address");
        let connect = TcpStream::connect(&address);
        Self { connect, tx }
    }
}
impl Future for Socket {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.connect.poll() {
            Ok(Async::Ready(socket)) => {
                self.tx.clone().try_send(socket);
                Ok(Async::Ready(()))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => {
                println!("failed to connect: {}", e);
                Ok(Async::Ready(()))
            }
        }
    }
}

pub struct PubSub {}

impl PubSub {
    pub fn from(timeline: impl std::fmt::Display, user: User) -> Receiver {
        while OPEN_CONNECTIONS.load(Ordering::Relaxed) > MAX_CONNECTIONS.load(Ordering::Relaxed) {
            thread::sleep(time::Duration::from_millis(1000));
        }
        let new_connections = OPEN_CONNECTIONS.fetch_add(1, Ordering::Relaxed) + 1;
        println!("{} connection(s) now open", new_connections);

        let (tx, mut rx) = tokio::sync::mpsc::channel(5);
        let socket = Socket::new("127.0.0.1:6379", tx);

        tokio::spawn(futures::future::lazy(move || socket));

        let socket = loop {
            if let Ok(Async::Ready(Some(msg))) = rx.poll() {
                break msg;
            }
            thread::sleep(time::Duration::from_millis(100));
        };

        let timeline = timeline.to_string();
        let stream_of_data_from_redis = Receiver::new(socket, timeline, user);
        stream_of_data_from_redis
    }
}
