use crate::{err, maybe_update};
use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, Ipv4Addr},
    os::unix::net::UnixListener,
    time::Duration,
};

#[derive(Debug)]
pub struct DeploymentConfig<'a> {
    pub env: String,
    pub log_level: String,
    pub address: IpAddr,
    pub port: u16,
    pub unix_socket: Option<UnixListener>,
    pub cors: Cors<'a>,
    pub sse_interval: Duration,
    pub ws_interval: Duration,
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

impl Default for DeploymentConfig<'_> {
    fn default() -> Self {
        Self {
            env: "development".to_string(),
            log_level: "error".to_string(),
            address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 4000,
            unix_socket: None,
            cors: Cors {
                allowed_methods: vec!["GET", "OPTIONS"],
                allowed_headers: vec!["Authorization", "Accept", "Cache-Control"],
            },
            sse_interval: Duration::from_millis(100),
            ws_interval: Duration::from_millis(100),
        }
    }
}
impl DeploymentConfig<'_> {
    pub fn from_env(env_vars: HashMap<String, String>) -> Self {
        Self::default()
            .maybe_update_env(env_vars.get("NODE_ENV").map(String::from))
            .maybe_update_env(env_vars.get("RUST_ENV").map(String::from))
            .maybe_update_address(
                env_vars
                    .get("BIND")
                    .map(|a| err::unwrap_or_die(a.parse().ok(), "BIND must be a valid address")),
            )
            .maybe_update_port(
                env_vars
                    .get("PORT")
                    .map(|port| err::unwrap_or_die(port.parse().ok(), "PORT must be a number")),
            )
            .maybe_update_unix_socket(
                env_vars
                    .get("SOCKET")
                    .map(|s| UnixListener::bind(s).unwrap()),
            )
            .maybe_update_log_level(env_vars.get("RUST_LOG").map(|level| match level.as_ref() {
                l @ "trace" | l @ "debug" | l @ "info" | l @ "warn" | l @ "error" => l.to_string(),
                _ => err::die_with_msg("Invalid log level specified"),
            }))
            .maybe_update_sse_interval(
                env_vars
                    .get("SSE_UPDATE_INTERVAL")
                    .map(|str| Duration::from_millis(str.parse().unwrap())),
            )
            .maybe_update_ws_interval(
                env_vars
                    .get("WS_UPDATE_INTERVAL")
                    .map(|str| Duration::from_millis(str.parse().unwrap())),
            )
            .log()
    }

    maybe_update!(maybe_update_env; env: String);
    maybe_update!(maybe_update_port; port: u16);
    maybe_update!(maybe_update_address; address: IpAddr);
    maybe_update!(maybe_update_unix_socket; Some(unix_socket: UnixListener));
    maybe_update!(maybe_update_log_level; log_level: String);
    maybe_update!(maybe_update_sse_interval; sse_interval: Duration);
    maybe_update!(maybe_update_ws_interval; ws_interval: Duration);

    fn log(self) -> Self {
        log::warn!("Using deployment configuration:\n {:#?}", &self);
        self
    }
}
