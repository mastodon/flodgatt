use futures::{Async, Future, Poll};
use regex::Regex;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite, Error, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use warp::Stream;

pub struct Receiver {
    rx: ReadHalf<TcpStream>,
}
impl Stream for Receiver {
    type Item = Value;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Value>, Self::Error> {
        let mut buffer = vec![0u8; 3000];
        if let Async::Ready(num_bytes_read) = self.rx.poll_read(&mut buffer)? {
            let re = Regex::new(r"(?x)(?P<json>\{.*\})").unwrap();

            if let Some(cap) = re.captures(&String::from_utf8_lossy(&buffer[..num_bytes_read])) {
                let json_string = cap["json"].to_string();
                let json: Value = serde_json::from_str(&json_string.clone())?;
                return Ok(Async::Ready(Some(json)));
            }
            return Ok(Async::NotReady);
        }
        Ok(Async::NotReady)
    }
}

struct Sender {
    tx: WriteHalf<TcpStream>,
    channel: String,
}
impl Future for Sender {
    type Item = ();
    type Error = Box<Error>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        println!("Subscribing to {}", &self.channel);
        let subscribe_cmd = format!(
            "*2\r\n$9\r\nsubscribe\r\n${}\r\n{}\r\n",
            self.channel.len(),
            self.channel
        );
        let buffer = subscribe_cmd.as_bytes();
        self.tx.poll_write(&buffer)?;
        Ok(Async::NotReady)
    }
}

fn get_socket() -> impl Future<Item = TcpStream, Error = Box<Error>> {
    let address = "127.0.0.1:6379".parse().expect("Unable to parse address");
    let connection = TcpStream::connect(&address);
    connection.and_then(Ok).map_err(Box::new)
}

fn send_subscribe_cmd(tx: WriteHalf<TcpStream>, channel: String) {
    let sender = Sender { tx, channel };
    tokio::spawn(sender.map_err(|e| eprintln!("{}", e)));
}

pub fn stream_from(
    timeline: String,
) -> impl Future<Item = Receiver, Error = warp::reject::Rejection> {
    get_socket()
        .and_then(move |socket| {
            let (rx, tx) = socket.split();
            send_subscribe_cmd(tx, format!("timeline:{}", timeline));
            let stream_of_data_from_redis = Receiver { rx };
            Ok(stream_of_data_from_redis)
        })
        .and_then(Ok)
        .map_err(warp::reject::custom)
}
