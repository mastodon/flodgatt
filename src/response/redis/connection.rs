mod err;
pub(crate) use err::RedisConnErr;

use super::msg::{RedisParseErr, RedisParseOutput};
use super::{ManagerErr, RedisCmd};
use crate::config::Redis;
use crate::event::Event;
use crate::request::{Stream, Timeline};

use futures::{Async, Poll};
use lru::LruCache;
use std::convert::{TryFrom, TryInto};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::str;
use std::time::Duration;

type Result<T> = std::result::Result<T, RedisConnErr>;

#[derive(Debug)]
pub(crate) struct RedisConn {
    primary: TcpStream,
    secondary: TcpStream,
    redis_namespace: Option<String>,
    tag_id_cache: LruCache<String, i64>,
    tag_name_cache: LruCache<i64, String>,
    redis_input: Vec<u8>,
    cursor: usize,
}

impl RedisConn {
    pub(crate) fn new(redis_cfg: &Redis) -> Result<Self> {
        let addr = [&*redis_cfg.host, ":", &*redis_cfg.port.to_string()].concat();

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
            redis_namespace: redis_cfg.namespace.clone().0,
            redis_input: vec![0_u8; 5000],
            cursor: 0,
        };
        Ok(redis_conn)
    }

    pub(crate) fn poll_redis(&mut self) -> Poll<Option<(Timeline, Event)>, ManagerErr> {
        loop {
            match self.primary.read(&mut self.redis_input[self.cursor..]) {
                Ok(n) => {
                    self.cursor += n;
                    if self.redis_input.len() - 1 == self.cursor {
                        self.redis_input.resize(self.redis_input.len() * 2, 0);
                    } else {
                        break;
                    }
                }
                Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock) => {
                    return Ok(Async::NotReady);
                }
                Err(e) => break log::error!("{}", e),
            };
        }

        // at this point, we have the raw bytes; now, parse a msg
        let input = &self.redis_input[..self.cursor];

        let (input, invalid_bytes) = str::from_utf8(&input)
            .map(|input| (input, &b""[..]))
            .unwrap_or_else(|e| {
                let (valid, invalid) = input.split_at(e.valid_up_to());
                (str::from_utf8(valid).expect("Guaranteed by ^^^^"), invalid)
            });

        use {Async::*, RedisParseOutput::*};
        let (res, leftover) = match RedisParseOutput::try_from(input) {
            Ok(Msg(msg)) => match &self.redis_namespace {
                Some(ns) if msg.timeline_txt.starts_with(&[ns, ":timeline:"].concat()) => {
                    let trimmed_tl = &msg.timeline_txt[ns.len() + ":timeline:".len()..];
                    let tl = Timeline::from_redis_text(trimmed_tl, &mut self.tag_id_cache)?;
                    let event = msg.event_txt.try_into()?;
                    (Ok(Ready(Some((tl, event)))), (msg.leftover_input))
                }
                None => {
                    let trimmed_tl = &msg.timeline_txt["timeline:".len()..];
                    let tl = Timeline::from_redis_text(trimmed_tl, &mut self.tag_id_cache)?;
                    let event = msg.event_txt.try_into()?;
                    (Ok(Ready(Some((tl, event)))), (msg.leftover_input))
                }
                Some(_non_matching_namespace) => (Ok(Ready(None)), msg.leftover_input),
            },
            Ok(NonMsg(leftover)) => (Ok(Ready(None)), leftover),
            Err(RedisParseErr::Incomplete) => (Ok(NotReady), input),
            Err(other_parse_err) => (Err(ManagerErr::RedisParseErr(other_parse_err)), input),
        };

        self.cursor = [leftover.as_bytes(), invalid_bytes]
            .concat()
            .bytes()
            .fold(0, |acc, cur| {
                // TODO - make clearer and comment side-effect
                self.redis_input[acc] = cur.expect("TODO");
                acc + 1
            });
        res
    }

    pub(crate) fn update_cache(&mut self, hashtag: String, id: i64) {
        self.tag_id_cache.put(hashtag.clone(), id);
        self.tag_name_cache.put(id, hashtag);
    }

    pub(crate) fn send_cmd(&mut self, cmd: RedisCmd, timeline: &Timeline) -> Result<()> {
        let hashtag = match timeline {
            Timeline(Stream::Hashtag(id), _, _) => self.tag_name_cache.get(id),
            _non_hashtag_timeline => None,
        };

        let tl = timeline.to_redis_raw_timeline(hashtag)?;
        let (primary_cmd, secondary_cmd) = cmd.into_sendable(&tl);
        self.primary.write_all(&primary_cmd)?;
        self.secondary.write_all(&secondary_cmd)?;
        Ok(())
    }

    fn new_connection(addr: &str, pass: Option<&String>) -> Result<TcpStream> {
        let mut conn = TcpStream::connect(&addr)?;
        if let Some(password) = pass {
            Self::auth_connection(&mut conn, &addr, password)?;
        }

        Self::validate_connection(&mut conn, &addr)?;
        conn.set_read_timeout(Some(Duration::from_millis(10)))
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        Ok(conn)
    }

    fn auth_connection(conn: &mut TcpStream, addr: &str, pass: &str) -> Result<()> {
        conn.write_all(
            &[
                b"*2\r\n$4\r\nauth\r\n$",
                pass.len().to_string().as_bytes(),
                b"\r\n",
                pass.as_bytes(),
                b"\r\n",
            ]
            .concat(),
        )
        .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let mut buffer = vec![0_u8; 5];
        conn.read_exact(&mut buffer)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        if String::from_utf8_lossy(&buffer) != "+OK\r\n" {
            Err(RedisConnErr::IncorrectPassword(pass.to_string()))?
        }
        Ok(())
    }

    fn validate_connection(conn: &mut TcpStream, addr: &str) -> Result<()> {
        conn.write_all(b"PING\r\n")
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let mut buffer = vec![0_u8; 7];
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
}
