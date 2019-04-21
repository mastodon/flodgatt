use crate::user::User;
use futures::{Async, Future, Poll};
use log::{debug, info};
use regex::Regex;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite, Error, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use warp::Stream;

pub struct Receiver {
    rx: ReadHalf<TcpStream>,
    tx: WriteHalf<TcpStream>,
    timeline: String,
    pub user: User,
}
impl Receiver {
    fn new(socket: TcpStream, timeline: String, user: User) -> Self {
        let (rx, mut tx) = socket.split();
        let channel = format!("timeline:{}", timeline);
        info!("Subscribing to {}", &channel);
        let subscribe_cmd = format!(
            "*2\r\n$9\r\nsubscribe\r\n${}\r\n{}\r\n",
            channel.len(),
            channel
        );
        let buffer = subscribe_cmd.as_bytes();
        tx.poll_write(&buffer).unwrap();
        Self {
            rx,
            tx,
            timeline,
            user,
        }
    }
}
impl Stream for Receiver {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let mut buffer = vec![0u8; 3000];
        if let Async::Ready(num_bytes_read) = self.rx.poll_read(&mut buffer)? {
            let re = Regex::new(r"(?x)(?P<json>\{.*\})").unwrap();

            if let Some(cap) = re.captures(&String::from_utf8_lossy(&buffer[..num_bytes_read])) {
                debug!("{}", &cap["json"]);
                let json_string = cap["json"].to_string();
                let json: Value = serde_json::from_str(&json_string.clone())?;
                return Ok(Async::Ready(Some(json)));
            }
            return Ok(Async::NotReady);
        }
        Ok(Async::NotReady)
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let channel = format!("timeline:{}", self.timeline);
        let unsubscribe_cmd = format!(
            "*2\r\n$9\r\nsubscribe\r\n${}\r\n{}\r\n",
            channel.len(),
            channel
        );
        self.tx.poll_write(unsubscribe_cmd.as_bytes()).unwrap();
        println!("Receiver got dropped!");
    }
}

fn get_socket() -> impl Future<Item = TcpStream, Error = Box<Error>> {
    let address = "127.0.0.1:6379".parse().expect("Unable to parse address");
    let connection = TcpStream::connect(&address);
    connection.and_then(Ok).map_err(Box::new)
}

pub fn stream_from(
    timeline: String,
    user: User,
) -> impl Future<Item = Receiver, Error = warp::reject::Rejection> {
    get_socket()
        .and_then(move |socket| {
            let stream_of_data_from_redis = Receiver::new(socket, timeline, user);
            Ok(stream_of_data_from_redis)
        })
        .map_err(warp::reject::custom)
}
