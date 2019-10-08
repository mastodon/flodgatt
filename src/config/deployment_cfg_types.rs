use crate::from_env_var;
use std::{fmt, net::IpAddr, os::unix::net::UnixListener, str::FromStr, time::Duration};
use strum_macros::{EnumString, EnumVariantNames};

from_env_var!(/// The current environment, which controls what file to read other ENV vars from
    Env {
        inner: EnvInner::Development; EnvInner,
        env_var: "RUST_ENV",
        allowed_values: format!("one of: {:?}", EnvInner::variants()),
    }
    inner_from_str(|s| EnvInner::from_str(s).ok())
);
#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum EnvInner {
    Production,
    Development,
}

from_env_var!(/// The address to run Flodgatt on
    FlodgattAddr {
        inner: IpAddr::V4("127.0.0.1".parse().expect("hardcoded")); IpAddr,
        env_var: "BIND",
        allowed_values: "a valid address (e.g., 127.0.0.1)".to_string(),
    }
    inner_from_str(|s| s.parse().ok()));
from_env_var!(/// How verbosely Flodgatt should log messages
    LogLevel {
        inner: LogLevelInner::Warn; LogLevelInner,
        env_var: "RUST_LOG",
        allowed_values: format!("one of {:?}", LogLevelInner::variants()),
    }
    inner_from_str(|s| LogLevelInner::from_str(s).ok()));
#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum LogLevelInner {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
from_env_var!(/// A Unix Socket to use in place of a local address
    Socket{
        inner: None; Option<UnixListener>,
        env_var: "SOCKET",
        allowed_values: "a valid Unix Socket".to_string(),
    }
    inner_from_str(|s| match UnixListener::bind(s).ok() {
        Some(socket) => Some(Some(socket)),
        None => None,
    }));
from_env_var!(/// The time between replies sent via WebSocket
    WsInterval {
        inner: Duration::from_millis(100); Duration,
        env_var: "WS_FREQ",
        allowed_values: "a number of milliseconds".to_string(),
    }
    inner_from_str(|s| s.parse().map(|num| Duration::from_millis(num)).ok()));
from_env_var!(/// The time between replies sent via Server Sent Events
    SseInterval {
        inner: Duration::from_millis(100); Duration,
        env_var: "SSE_FREQ",
        allowed_values: "a number of milliseconds".to_string(),
    }
    inner_from_str(|s| s.parse().map(|num| Duration::from_millis(num)).ok()));
from_env_var!(/// The port to run Flodgatt on
    Port2 {
        inner: 4000; u16,
        env_var: "PORT",
        allowed_values: "a number".to_string(),
    }
    inner_from_str(|s| s.parse().ok()));

/// Permissions for Cross Origin Resource Sharing (CORS)
pub struct Cors<'a> {
    pub allowed_headers: Vec<&'a str>,
    pub allowed_methods: Vec<&'a str>,
}
impl fmt::Debug for Cors<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "allowed headers: {:?}\n      allowed methods: {:?}",
            self.allowed_headers, self.allowed_methods
        )
    }
}
impl std::default::Default for Cors<'_> {
    fn default() -> Self {
        Self {
            allowed_methods: vec!["GET", "OPTIONS"],
            allowed_headers: vec!["Authorization", "Accept", "Cache-Control"],
        }
    }
}
