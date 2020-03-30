use super::{
    redis_cmd,
    redis_msg::{RedisMsg, RedisParseOutput},
};
use crate::config::RedisConfig;
use crate::err::{self, RedisParseErr};
use crate::messages::Event;
use crate::parse_client_request::Timeline;
use crate::pubsub_cmd;
use futures::{Async, Poll};
use lru::LruCache;
use std::{
    convert::TryFrom,
    io::Read,
    io::Write,
    net, str,
    time::{Duration, Instant},
};
use tokio::io::AsyncRead;

#[derive(Debug)]
pub struct RedisConn {
    primary: net::TcpStream,
    secondary: net::TcpStream,
    redis_poll_interval: Duration,
    redis_polled_at: Instant,
    redis_namespace: Option<String>,
    cache: LruCache<String, i64>,
    redis_input: Vec<u8>, // TODO: Consider queue internal to RedisConn
}

impl RedisConn {
    pub fn new(redis_cfg: RedisConfig) -> Self {
        let addr = format!("{}:{}", *redis_cfg.host, *redis_cfg.port);
        let conn_err = |e| {
            err::die_with_msg(format!(
                "Could not connect to Redis at {}:{}.\n             Error detail: {}",
                *redis_cfg.host, *redis_cfg.port, e,
            ))
        };
        let update_conn = |mut conn| {
            if let Some(password) = redis_cfg.password.clone() {
                conn = send_password(conn, &password);
            }
            conn = send_test_ping(conn);
            conn.set_read_timeout(Some(Duration::from_millis(10)))
                .expect("Can set read timeout for Redis connection");
            if let Some(db) = &*redis_cfg.db {
                conn = set_db(conn, db);
            }
            conn
        };
        let (primary_conn, secondary_conn) = (
            update_conn(net::TcpStream::connect(addr.clone()).unwrap_or_else(conn_err)),
            update_conn(net::TcpStream::connect(addr).unwrap_or_else(conn_err)),
        );
        primary_conn
            .set_nonblocking(true)
            .expect("set_nonblocking call failed");

        Self {
            primary: primary_conn,
            secondary: secondary_conn,
            cache: LruCache::new(1000),
            redis_namespace: redis_cfg.namespace.clone(),
            redis_poll_interval: *redis_cfg.polling_interval,
            redis_input: Vec::new(),
            redis_polled_at: Instant::now(),
        }
    }

    pub fn poll_redis(&mut self) -> Poll<Option<(Timeline, Event)>, RedisParseErr> {
        let mut buffer = vec![0u8; 6000];
        if self.redis_polled_at.elapsed() > self.redis_poll_interval {
            if let Ok(Async::Ready(bytes_read)) = self.poll_read(&mut buffer) {
                self.redis_input.extend_from_slice(&buffer[..bytes_read]);
            }
        }
        let input = self.redis_input.clone();
        self.redis_input.clear();
        let (input, invalid_bytes) = match str::from_utf8(&input) {
            Ok(input) => (input, None),
            Err(e) => {
                let (valid, invalid) = input.split_at(e.valid_up_to());
                (str::from_utf8(valid).expect("Guaranteed ^"), Some(invalid))
            }
        };

        let (cache, ns) = (&mut self.cache, &self.redis_namespace);

        use {Async::*, RedisParseOutput::*};

        let (res, leftover) = match RedisParseOutput::try_from(input) {
            Ok(Msg(msg)) => match ns {
                Some(ns) if msg.timeline_txt.starts_with(&format!("{}:timeline:", ns)) => {
                    let tl = Timeline::from_redis_text(
                        &msg.timeline_txt[ns.len() + ":timeline:".len()..],
                        cache,
                    )
                    .unwrap_or_else(|_| todo!());
                    let event: Event = serde_json::from_str(msg.event_txt).expect("TODO");
                    (Ok(Ready(Some((tl, event)))), msg.leftover_input)
                }
                None => {
                    let tl =
                        Timeline::from_redis_text(&msg.timeline_txt["timeline:".len()..], cache)
                            .unwrap_or_else(|_| todo!());

                    let event: Event = serde_json::from_str(msg.event_txt).expect("TODO");

                    (Ok(Ready(Some((tl, event)))), msg.leftover_input)
                }
                Some(_non_matching_namespace) => (Ok(Ready(None)), msg.leftover_input),
            },
            Ok(NonMsg(leftover)) => (Ok(Ready(None)), leftover),
            Err(RedisParseErr::Incomplete) => (Ok(NotReady), input),
            Err(_other) => todo!(),
        };
        self.redis_input.extend_from_slice(leftover.as_bytes());
        if let Some(bytes) = invalid_bytes {
            self.redis_input.extend_from_slice(bytes);
        };
        res
    }

    pub fn update_cache(&mut self, hashtag: String, id: i64) {
        self.cache.put(hashtag, id);
    }

    pub fn send_unsubscribe_cmd(&mut self, timeline: &str) {
        pubsub_cmd!("unsubscribe", self, timeline);
    }
    pub fn send_subscribe_cmd(&mut self, timeline: &str) {
        pubsub_cmd!("subscribe", self, timeline);
    }
}

fn send_password(mut conn: net::TcpStream, password: &str) -> net::TcpStream {
    conn.write_all(&redis_cmd::cmd("auth", &password)).unwrap();
    let mut buffer = vec![0u8; 5];
    conn.read_exact(&mut buffer).unwrap();
    let reply = String::from_utf8(buffer.to_vec()).unwrap();
    if reply != "+OK\r\n" {
        err::die_with_msg(format!(
            r"Incorrect Redis password.  You supplied `{}`.
             Please supply correct password with REDIS_PASSWORD environmental variable.",
            password,
        ))
    };
    conn
}

fn set_db(mut conn: net::TcpStream, db: &str) -> net::TcpStream {
    conn.write_all(&redis_cmd::cmd("SELECT", &db)).unwrap();
    conn
}

fn send_test_ping(mut conn: net::TcpStream) -> net::TcpStream {
    conn.write_all(b"PING\r\n").unwrap();
    let mut buffer = vec![0u8; 7];
    conn.read_exact(&mut buffer).unwrap();
    let reply = String::from_utf8(buffer.to_vec()).unwrap();
    match reply.as_str() {
        "+PONG\r\n" => (),
        "-NOAUTH" => err::die_with_msg(
            r"Invalid authentication for Redis.
             Redis reports that it needs a password, but you did not provide one.
             You can set a password with the REDIS_PASSWORD environmental variable.",
        ),
        "HTTP/1." => err::die_with_msg(
            r"The server at REDIS_HOST and REDIS_PORT is not a Redis server.
             Please update the REDIS_HOST and/or REDIS_PORT environmental variables.",
        ),
        _ => err::die_with_msg(format!(
            "Could not connect to Redis for unknown reason.  Expected `+PONG` reply but got {}",
            reply
        )),
    };
    conn
}

impl Read for RedisConn {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.primary.read(buffer)
    }
}

impl AsyncRead for RedisConn {
    fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, std::io::Error> {
        match self.read(buf) {
            Ok(t) => Ok(Async::Ready(t)),
            Err(_) => Ok(Async::NotReady),
        }
    }
}
