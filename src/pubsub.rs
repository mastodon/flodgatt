use crate::stream;
use crate::user::User;
use futures::{Async, Future, Poll};
use log::info;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{thread, time};
use tokio::net::TcpStream;
use warp::Stream;

pub static OPEN_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
pub static MAX_CONNECTIONS: AtomicUsize = AtomicUsize::new(400);

pub struct RedisCmd {
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
    pub fn subscribe_to_timeline(timeline: &str) -> String {
        let channel = format!("timeline:{}", timeline);
        let subscribe = RedisCmd::new("subscribe", &channel);
        info!("Subscribing to {}", &channel);
        subscribe.resp_cmd
    }
    pub fn unsubscribe_from_timeline(timeline: &str) -> String {
        let channel = format!("timeline:{}", timeline);
        let unsubscribe = RedisCmd::new("unsubscribe", &channel);
        info!("Unsubscribing from {}", &channel);
        unsubscribe.resp_cmd
    }
}

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
                self.tx.clone().try_send(socket).expect("Socket created");
                Ok(Async::Ready(()))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => {
                info!("failed to connect: {}", e);
                Ok(Async::Ready(()))
            }
        }
    }
}

pub struct PubSub {}

impl PubSub {
    pub fn from(timeline: impl std::fmt::Display, user: &User) -> stream::Receiver {
        while OPEN_CONNECTIONS.load(Ordering::Relaxed) > MAX_CONNECTIONS.load(Ordering::Relaxed) {
            thread::sleep(time::Duration::from_millis(1000));
        }
        let new_connections = OPEN_CONNECTIONS.fetch_add(1, Ordering::Relaxed) + 1;
        info!("{} connection(s) now open", new_connections);

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
        let stream_of_data_from_redis = stream::Receiver::new(socket, timeline, user);
        stream_of_data_from_redis
    }
}
