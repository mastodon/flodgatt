mod err;
pub(crate) use err::RedisConnErr;

use super::Error as ManagerErr;
use super::RedisCmd;
use crate::config::Redis;
use crate::request::Timeline;

use futures::{Async, Poll};
use lru::LruCache;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

type Result<T> = std::result::Result<T, RedisConnErr>;

#[derive(Debug)]
pub(super) struct RedisConn {
    primary: TcpStream,
    secondary: TcpStream,
    pub(super) namespace: Option<String>,
    // TODO: eventually, it might make sense to have Mastodon publish to timelines with
    //       the tag number instead of the tag name.  This would save us from dealing
    //       with a cache here and would be consistent with how lists/users are handled.
    pub(super) tag_id_cache: LruCache<String, i64>,
    tag_name_cache: LruCache<i64, String>,
    pub(super) input: Vec<u8>,
}

impl RedisConn {
    pub(super) fn new(redis_cfg: &Redis) -> Result<Self> {
        let addr = [&*redis_cfg.host, ":", &*redis_cfg.port.to_string()].concat();

        let conn = Self::new_connection(&addr, redis_cfg.password.as_ref())?;
        conn.set_nonblocking(true)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        Ok(Self {
            primary: conn,
            secondary: Self::new_connection(&addr, redis_cfg.password.as_ref())?,
            tag_id_cache: LruCache::new(1000),
            tag_name_cache: LruCache::new(1000),
            namespace: redis_cfg.namespace.clone().0,
            input: vec![47_u8; 10_000], // TODO - set to something reasonable
        })
    }
    pub(super) fn poll_redis(&mut self, start: usize) -> Poll<usize, ManagerErr> {
        const BLOCK: usize = 8192;
        if self.input.len() <= start + BLOCK {
            self.input.resize(self.input.len() * 2, 0);
            log::info!("Resizing input buffer. (Old input was {} bytes)", start);
        }

        use Async::*;
        match self.primary.read(&mut self.input[start..start + BLOCK]) {
            Ok(n) => Ok(Ready(n)),
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock) => Ok(NotReady),
            Err(e) => {
                Ready(log::error!("{}", e));
                Ok(Ready(0))
            }
        }
    }

    pub(super) fn update_cache(&mut self, hashtag: String, id: i64) {
        self.tag_id_cache.put(hashtag.clone(), id);
        self.tag_name_cache.put(id, hashtag);
    }

    pub(crate) fn send_cmd(&mut self, cmd: RedisCmd, timelines: &[Timeline]) -> Result<()> {
        let namespace = self.namespace.take();
        let timelines: Result<Vec<String>> = timelines
            .iter()
            .map(|tl| {
                let hashtag = tl.tag().and_then(|id| self.tag_name_cache.get(&id));
                match &namespace {
                    Some(ns) => Ok(format!("{}:{}", ns, tl.to_redis_raw_timeline(hashtag)?)),
                    None => Ok(tl.to_redis_raw_timeline(hashtag)?),
                }
            })
            .collect();

        let (primary_cmd, secondary_cmd) = cmd.into_sendable(&timelines?[..]);
        self.primary.write_all(&primary_cmd)?;

        // We also need to set a key to tell the Puma server that we've subscribed or
        // unsubscribed to the channel because it stops publishing updates when it thinks
        // no one is subscribed.
        // (Documented in [PR #3278](https://github.com/tootsuite/mastodon/pull/3278))
        // Question: why can't the Puma server just use NUMSUB for this?
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
        Self::set_connection_name(&mut conn, &addr)?;
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
        let mut buffer = vec![0_u8; 100];
        conn.read(&mut buffer)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let reply = String::from_utf8_lossy(&buffer);
        match &*reply {
            r if r.starts_with("+PONG\r\n") => Ok(()),
            r if r.starts_with("-NOAUTH") => Err(RedisConnErr::MissingPassword),
            r if r.starts_with("HTTP/1.") => Err(RedisConnErr::NotRedis(addr.to_string())),
            _ => Err(RedisConnErr::InvalidRedisReply(reply.to_string())),
        }
    }

    fn set_connection_name(conn: &mut TcpStream, addr: &str) -> Result<()> {
        conn.write_all(b"*3\r\n$6\r\nCLIENT\r\n$7\r\nSETNAME\r\n$8\r\nflodgatt\r\n")
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let mut buffer = vec![0_u8; 100];
        conn.read(&mut buffer)
            .map_err(|e| RedisConnErr::with_addr(&addr, e))?;
        let reply = String::from_utf8_lossy(&buffer);
        match &*reply {
            r if r.starts_with("+OK\r\n") => Ok(()),
            _ => Err(RedisConnErr::InvalidRedisReply(reply.to_string())),
        }
    }
}
