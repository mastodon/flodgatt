use crate::from_env_var;
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
    time::Duration,
};
use strum_macros::{EnumString, EnumVariantNames};

from_env_var!(
    /// The current environment, which controls what file to read other ENV vars from 
    let name = Env;
    let default: EnvInner = EnvInner::Development;
    let (env_var, allowed_values) = ("RUST_ENV",  &format!("one of: {:?}", EnvInner::variants()));
    let from_str = |s| EnvInner::from_str(s).ok();
);
from_env_var!(
    /// The address to run Flodgatt on
    let name = FlodgattAddr;
    let default: IpAddr = IpAddr::V4("127.0.0.1".parse().expect("hardcoded"));
    let (env_var, allowed_values) = ("BIND", "a valid address (e.g., 127.0.0.1)");
    let from_str = |s| match s {
        "localhost" => Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        _ => s.parse().ok(),
    };
);
from_env_var!(
    /// How verbosely Flodgatt should log messages
    let name = LogLevel;
    let default: LogLevelInner = LogLevelInner::Warn;
    let (env_var, allowed_values) = ("RUST_LOG",  &format!("one of: {:?}", LogLevelInner::variants())); 
    let from_str = |s| LogLevelInner::from_str(s).ok();
);
from_env_var!(
    /// A Unix Socket to use in place of a local address
    let name = Socket;
    let default: Option<String> = None;
    let (env_var, allowed_values) = ("SOCKET", "any string");
    let from_str = |s| Some(Some(s.to_string()));
);
from_env_var!(
    /// The time between replies sent via WebSocket
    let name = WsInterval;
    let default: Duration = Duration::from_millis(100);
    let (env_var, allowed_values) = ("WS_FREQ",  "a number of milliseconds");
    let from_str = |s| s.parse().map(Duration::from_millis).ok();
);
from_env_var!(
    /// The time between replies sent via Server Sent Events
    let name = SseInterval;
    let default: Duration = Duration::from_millis(100);
    let (env_var, allowed_values) = ("WS_FREQ", "a number of milliseconds");
    let from_str = |s| s.parse().map(Duration::from_millis).ok();
);
from_env_var!(
    /// The port to run Flodgatt on
    let name = Port;
    let default: u16 = 4000;
    let (env_var, allowed_values) = ("PORT", "a number between 0 and 65535");
    let from_str = |s| s.parse().ok();
);
from_env_var!(
    /// Enables [WHITELIST_MODE](https://docs.joinmastodon.org/admin/config/#whitelist_mode)
    ///
    /// This mode prevents non-logged in users from subscribing to any timelines
    /// (including otherwise public timelines).
    let name = WhitelistMode;
    let default: bool = false;
    let (env_var, allowed_values) = ("WHITELIST_MODE", "true or false");
    let from_str = |s| s.parse().ok();
);
/// Permissions for Cross Origin Resource Sharing (CORS)
pub struct Cors<'a> {
    pub allowed_headers: Vec<&'a str>,
    pub allowed_methods: Vec<&'a str>,
}
impl std::default::Default for Cors<'_> {
    fn default() -> Self {
        Self {
            allowed_methods: vec!["GET", "OPTIONS"],
            allowed_headers: vec!["Authorization", "Accept", "Cache-Control"],
        }
    }
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

#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum LogLevelInner {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(EnumString, EnumVariantNames, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum EnvInner {
    Production,
    Development,
}
