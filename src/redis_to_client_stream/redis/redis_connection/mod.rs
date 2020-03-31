mod err;
pub use err::RedisConnErr;

use super::super::receiver::ReceiverErr;
use super::{
    redis_cmd,
    redis_msg::{RedisParseErr, RedisParseOutput},
};
use crate::{config::RedisConfig, messages::Event, parse_client_request::Timeline, pubsub_cmd};

use std::{
    convert::TryFrom,
    io::{Read, Write},
    net::TcpStream,
    str,
    time::{Duration, Instant},
};

use futures::{Async, Poll};
use lru::LruCache;

#[derive(Debug)]
pub struct RedisConn {
    primary: TcpStream,
    //    secondary: TcpStream,
    redis_poll_interval: Duration,
    redis_polled_at: Instant,
    redis_namespace: Option<String>,
    cache: LruCache<String, i64>,
    redis_input: Vec<u8>,
}

impl RedisConn {
    pub fn new(redis_cfg: RedisConfig) -> Result<Self, RedisConnErr> {
        let addr = format!("{}:{}", *redis_cfg.host, *redis_cfg.port);
        let password = redis_cfg.password.as_ref();

        let primary_conn = Self::new_connection(&addr, &password)?;

        primary_conn
            .set_nonblocking(true)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;

        Ok(Self {
            primary: primary_conn,
            //          secondary: Self::new_connection(&addr, &password)?,
            cache: LruCache::new(1000),
            redis_namespace: redis_cfg.namespace.clone(),
            redis_poll_interval: *redis_cfg.polling_interval,
            redis_input: Vec::new(),
            redis_polled_at: Instant::now(),
        })
    }

    pub fn poll_redis(&mut self) -> Poll<Option<(Timeline, Event)>, ReceiverErr> {
        let mut buffer = vec![0u8; 6000];
        if self.redis_polled_at.elapsed() > self.redis_poll_interval {
            match self.primary.read(&mut buffer) {
                Ok(bytes_read) => self.redis_input.extend_from_slice(&buffer[..bytes_read]),
                Err(e) => log::error!("Error polling Redis: {}\nRetrying...", e),
            }
        }
        let input = self.redis_input.clone();
        self.redis_input.clear();

        let (input, invalid_bytes) = str::from_utf8(&input)
            .map(|input| (input, "".as_bytes()))
            .unwrap_or_else(|e| {
                let (valid, invalid) = input.split_at(e.valid_up_to());
                (str::from_utf8(valid).expect("Guaranteed by ^^^^"), invalid)
            });

        use {Async::*, RedisParseOutput::*};
        let (res, leftover) = match RedisParseOutput::try_from(input) {
            Ok(Msg(msg)) => match &self.redis_namespace {
                Some(ns) if msg.timeline_txt.starts_with(&format!("{}:timeline:", ns)) => {
                    let trimmed_tl_txt = &msg.timeline_txt[ns.len() + ":timeline:".len()..];
                    let tl = Timeline::from_redis_text(trimmed_tl_txt, &mut self.cache)?;
                    let event: Event = serde_json::from_str(msg.event_txt)?;
                    (Ok(Ready(Some((tl, event)))), msg.leftover_input)
                }
                None => {
                    let trimmed_tl_txt = &msg.timeline_txt["timeline:".len()..];
                    let tl = Timeline::from_redis_text(trimmed_tl_txt, &mut self.cache)?;
                    let event: Event = serde_json::from_str(msg.event_txt)?;
                    (Ok(Ready(Some((tl, event)))), msg.leftover_input)
                }
                Some(_non_matching_namespace) => (Ok(Ready(None)), msg.leftover_input),
            },
            Ok(NonMsg(leftover)) => (Ok(Ready(None)), leftover),
            Err(RedisParseErr::Incomplete) => (Ok(NotReady), input),
            Err(other_parse_err) => (Err(ReceiverErr::RedisParseErr(other_parse_err)), input),
        };
        self.redis_input.extend_from_slice(leftover.as_bytes());
        self.redis_input.extend_from_slice(invalid_bytes);
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

    fn new_connection(addr: &String, pass: &Option<&String>) -> Result<TcpStream, RedisConnErr> {
        match TcpStream::connect(&addr) {
            Ok(mut conn) => {
                if let Some(password) = pass {
                    Self::auth_connection(&mut conn, &addr, password)?;
                }

                Self::validate_connection(&mut conn, &addr)?;
                conn.set_read_timeout(Some(Duration::from_millis(10)))
                    .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
                Ok(conn)
            }
            Err(e) => Err(RedisConnErr::with_addr(&addr, e)),
        }
    }
    fn auth_connection(conn: &mut TcpStream, addr: &str, pass: &str) -> Result<(), RedisConnErr> {
        conn.write_all(&redis_cmd::cmd("auth", pass))
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let mut buffer = vec![0u8; 5];
        conn.read_exact(&mut buffer)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let reply = String::from_utf8_lossy(&buffer);
        match &*reply {
            "+OK\r\n" => (),
            _ => Err(RedisConnErr::IncorrectPassword(pass.to_string()))?,
        };
        Ok(())
    }

    fn validate_connection(conn: &mut TcpStream, addr: &str) -> Result<(), RedisConnErr> {
        conn.write_all(b"PING\r\n")
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let mut buffer = vec![0u8; 7];
        conn.read_exact(&mut buffer)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let reply = String::from_utf8_lossy(&buffer);
        match &*reply {
            "+PONG\r\n" => Ok(()),
            "-NOAUTH" => Err(RedisConnErr::MissingPassword),
            "HTTP/1." => Err(RedisConnErr::NotRedis(addr.to_string())),
            _ => Err(RedisConnErr::UnknownRedisErr(reply.to_string())),
        }
    }
}
