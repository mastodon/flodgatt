use crate::{derive_from_str_or_die, err::FromStrOrDie, from_env_var};
use std::{fmt, net::IpAddr, os::unix::net::UnixListener, str::FromStr, time::Duration};
use strum_macros::{EnumString, EnumVariantNames};

/// The current environment, which controls what file to read other ENV vars from
#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum Env {
    Production,
    Development,
}
derive_from_str_or_die!(Env {
    name: "RUST_ENV",
    value: format!("one of: {:?}", Self::variants())
});

// The address to run Flodgatt on
from_env_var!(FlodgattAddr {
    inner: IpAddr::V4("127.0.0.1".parse().expect("hardcoded")); IpAddr,
    env_var: "BIND",
    allowed_values: "a valid address (e.g., 127.0.0.1)".to_string(),
}
inner_from_str(|s| s.parse().ok()));

/// How verbosely Flodgatt should log messages
#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum LogLevelInner {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
from_env_var!(LogLevel {
    inner: LogLevelInner::Warn; LogLevelInner,
    env_var: "RUST_LOG",
    allowed_values: format!("one of {:?}", LogLevelInner::variants()),
}
inner_from_str(|s| LogLevelInner::from_str(s).ok()));

// A Unix Socket to use in place of a local address
from_env_var!(Socket{
    inner: None; Option<UnixListener>,
    env_var: "SOCKET",
    allowed_values: "a valid Unix Socket".to_string(),
}
inner_from_str(|s| match UnixListener::bind(s).ok() {
    Some(socket) => Some(Some(socket)),
    None => None,
}));

// The time between replies sent via WebSocket
from_env_var!(WsInterval {
    inner: Duration::from_millis(100); Duration,
    env_var: "WS_FREQ",
    allowed_values: "a number of milliseconds".to_string(),
}
inner_from_str(|s| s.parse().map(|num| Duration::from_millis(num)).ok()));

// The time between replies sent via Server Sent Events
from_env_var!(SseInterval {
    inner: Duration::from_millis(100); Duration,
    env_var: "SSE_FREQ",
    allowed_values: "a number of milliseconds".to_string(),
}
inner_from_str(|s| s.parse().map(|num| Duration::from_millis(num)).ok()));

// The port to run Flodgatt on
from_env_var!(Port2 {
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
