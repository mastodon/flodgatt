mod err;
pub use err::RedisConnErr;

use super::super::receiver::ReceiverErr;
use super::redis_msg::{RedisParseErr, RedisParseOutput};
use crate::{
    config::RedisConfig,
    messages::Event,
    parse_client_request::{Stream, Timeline},
};

use std::{
    convert::TryFrom,
    io::{Read, Write},
    net::TcpStream,
    str,
    time::Duration,
};

use futures::{Async, Poll};
use lru::LruCache;

type Result<T> = std::result::Result<T, RedisConnErr>;

#[derive(Debug)]
pub struct RedisConn {
    primary: TcpStream,
    secondary: TcpStream,
    redis_namespace: Option<String>,
    tag_id_cache: LruCache<String, i64>,
    tag_name_cache: LruCache<i64, String>,
    redis_input: Vec<u8>,
}

impl RedisConn {
    pub fn new(redis_cfg: RedisConfig) -> Result<Self> {
        let addr = format!("{}:{}", *redis_cfg.host, *redis_cfg.port);
        let conn = Self::new_connection(&addr, redis_cfg.password.as_ref())?;
        conn.set_nonblocking(true)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let redis_conn = Self {
            primary: conn,
            secondary: Self::new_connection(&addr, redis_cfg.password.as_ref())?,
            tag_id_cache: LruCache::new(1000),
            tag_name_cache: LruCache::new(1000),
            // TODO: eventually, it might make sense to have Mastodon publish to timelines with
            //       the tag number instead of the tag name.  This would save us from dealing
            //       with a cache here and would be consistent with how lists/users are handled.
            redis_namespace: redis_cfg.namespace.clone(),
            redis_input: Vec::new(),
        };
        Ok(redis_conn)
    }

    pub fn poll_redis(&mut self) -> Poll<Option<(Timeline, Event)>, ReceiverErr> {
        let mut size = 100; // large enough to handle subscribe/unsubscribe notice
        let (mut buffer, mut first_read) = (vec![0u8; size], true);
        loop {
            match self.primary.read(&mut buffer) {
                Ok(n) if n != size => {
                    self.redis_input.extend_from_slice(&buffer[..n]);
                    break;
                }
                Ok(n) => {
                    self.redis_input.extend_from_slice(&buffer[..n]);
                }
                Err(_) => break,
            };
            if first_read {
                size = 2000;
                buffer = vec![0u8; size];
                first_read = false;
            }
        }

        if self.redis_input.is_empty() {
            return Ok(Async::NotReady);
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
                    let tl = Timeline::from_redis_text(trimmed_tl_txt, &mut self.tag_id_cache)?;
                    let event = msg.event_txt.into();
                    (Ok(Ready(Some((tl, event)))), msg.leftover_input)
                }
                None => {
                    let trimmed_tl_txt = &msg.timeline_txt["timeline:".len()..];
                    let tl = Timeline::from_redis_text(trimmed_tl_txt, &mut self.tag_id_cache)?;
                    let event = msg.event_txt.into();
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
        self.tag_id_cache.put(hashtag.clone(), id);
        self.tag_name_cache.put(id, hashtag);
    }

    fn new_connection(addr: &str, pass: Option<&String>) -> Result<TcpStream> {
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
    fn auth_connection(conn: &mut TcpStream, addr: &str, pass: &str) -> Result<()> {
        conn.write_all(&format!("*2\r\n$4\r\nauth\r\n${}\r\n{}\r\n", pass.len(), pass).as_bytes())
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

    fn validate_connection(conn: &mut TcpStream, addr: &str) -> Result<()> {
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
            _ => Err(RedisConnErr::InvalidRedisReply(reply.to_string())),
        }
    }

    pub fn send_cmd(&mut self, cmd: RedisCmd, timeline: &Timeline) -> Result<()> {
        let hashtag = match timeline {
            Timeline(Stream::Hashtag(id), _, _) => self.tag_name_cache.get(id),
            _non_hashtag_timeline => None,
        };
        let tl = timeline.to_redis_raw_timeline(hashtag);

        let (primary_cmd, secondary_cmd) = match cmd {
            RedisCmd::Subscribe => (
                format!("*2\r\n$9\r\nsubscribe\r\n${}\r\n{}\r\n", tl.len(), tl),
                format!("*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n$1\r\n1\r\n", tl.len(), tl),
            ),
            RedisCmd::Unsubscribe => (
                format!("*2\r\n$11\r\nunsubscribe\r\n${}\r\n{}\r\n", tl.len(), tl),
                format!("*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n$1\r\n0\r\n", tl.len(), tl),
            ),
        };
        self.primary.write_all(&primary_cmd.as_bytes())?;
        self.secondary.write_all(&secondary_cmd.as_bytes())?;
        Ok(())
    }
}

pub enum RedisCmd {
    Subscribe,
    Unsubscribe,
}
