use crate::err::FromStrOrDie;
use std::{
    fmt, net::IpAddr, ops::Deref, os::unix::net::UnixListener, str::FromStr, time::Duration,
};
use strum_macros::{EnumString, EnumVariantNames};

#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum Env {
    Production,
    Development,
}
impl FromStrOrDie<Self> for Env {
    fn name_and_values() -> (&'static str, String) {
        ("RUST_ENV", format!("one of: {:?}", Self::variants()))
    }
}

pub struct FlodgattAddr(pub IpAddr);
impl Deref for FlodgattAddr {
    type Target = IpAddr;
    fn deref(&self) -> &IpAddr {
        &self.0
    }
}
impl FromStr for FlodgattAddr {
    type Err = std::net::AddrParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|num| Self(num))
    }
}
impl FromStrOrDie<Self> for FlodgattAddr {
    fn name_and_values() -> (&'static str, String) {
        ("BIND", "a valid address (e.g., 127.0.0.1)".to_string())
    }
}
impl fmt::Debug for FlodgattAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addr = match self.0 {
            IpAddr::V4(addr) => addr.to_string(),
            IpAddr::V6(addr) => addr.to_string(),
        };
        write!(f, "{}", addr)
    }
}

#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
impl FromStrOrDie<Self> for LogLevel {
    fn name_and_values() -> (&'static str, String) {
        ("RUST_LOG", format!("one of: {:?}", Self::variants()))
    }
}

#[derive(Debug)]
pub struct Socket(pub UnixListener);
impl Deref for Socket {
    type Target = UnixListener;
    fn deref(&self) -> &UnixListener {
        &self.0
    }
}
impl FromStr for Socket {
    type Err = std::io::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        UnixListener::bind(s).map(|socket| (Self(socket)))
    }
}
impl FromStrOrDie<Self> for Socket {
    fn name_and_values() -> (&'static str, String) {
        ("SOCKET", "a valid Unix Socket".to_string())
    }
}

/// The time between replies sent via WebSocket
pub struct WsInterval(pub Duration);
impl Deref for WsInterval {
    type Target = Duration;
    fn deref(&self) -> &Duration {
        &self.0
    }
}
impl FromStr for WsInterval {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|num| Self(Duration::from_millis(num)))
    }
}
impl FromStrOrDie<Self> for WsInterval {
    fn name_and_values() -> (&'static str, String) {
        ("WS_FREQ", "a number of milliseconds".to_string())
    }
}
impl fmt::Debug for WsInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// The time between replies sent via Server Sent Events
pub struct SseInterval(pub Duration);
impl Deref for SseInterval {
    type Target = Duration;
    fn deref(&self) -> &Duration {
        &self.0
    }
}
impl FromStr for SseInterval {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(|num| Self(Duration::from_millis(num)))
    }
}
impl FromStrOrDie<Self> for SseInterval {
    fn name_and_values() -> (&'static str, String) {
        ("SSE_FREQ", "a number of milliseconds".to_string())
    }
}
impl fmt::Debug for SseInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl FromStrOrDie<Self> for u16 {
    fn name_and_values() -> (&'static str, String) {
        ("PORT", "a number".to_string())
    }
}

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
