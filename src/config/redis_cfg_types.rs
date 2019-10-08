use crate::from_env_var;
use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
//use std::{fmt, net::IpAddr, os::unix::net::UnixListener, str::FromStr, time::Duration};
//use strum_macros::{EnumString, EnumVariantNames};

from_env_var!(
    /// The host address where Redis is running
    let name = RedisHost;
    let default: IpAddr = IpAddr::V4("127.0.0.1".parse().expect("hardcoded"));
    let (env_var, allowed_values) = ("REDIS_HOST", "a valid address (e.g., 127.0.0.1)".to_string());
    let from_str = |s| match s {
        "localhost" => Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        _ => s.parse().ok(),
    };
);

from_env_var!(
    /// The port Redis is running on
    let name = RedisPort;
    let default: u16 = 6379;
    let (env_var, allowed_values) = ("REDIS_PORT", "a number between 0 and 65535".to_string());
    let from_str = |s| s.parse().ok();
);
from_env_var!(
    /// How frequently to poll Redis
    let name = RedisInterval;
    let default: Duration = Duration::from_millis(100);
    let (env_var, allowed_values) = ("REDIS_POLL_INTERVAL", "a number of milliseconds".to_string());
    let from_str = |s| s.parse().map(|num| Duration::from_millis(num)).ok();
);
from_env_var!(
    /// The password to use for Redis
    let name = RedisPass;
    let default: Option<String> = None;
    let (env_var, allowed_values) = ("REDIS_PASSWORD", "any string".to_string());
    let from_str = |s| Some(Some(s.to_string()));
);
