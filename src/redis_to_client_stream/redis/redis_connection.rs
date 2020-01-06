use super::redis_cmd;
use crate::config::{RedisConfig, RedisInterval, RedisNamespace};
use crate::err;
use std::{io::Read, io::Write, net, time};

pub struct RedisConn {
    pub primary: net::TcpStream,
    pub secondary: net::TcpStream,
    pub namespace: RedisNamespace,
    pub polling_interval: RedisInterval,
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
            conn.set_read_timeout(Some(time::Duration::from_millis(10)))
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
            namespace: redis_cfg.namespace,
            polling_interval: redis_cfg.polling_interval,
        }
    }
}
