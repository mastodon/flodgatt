use crate::{config::deployment_cfg_types::*, err::FromStrOrDie, maybe_update};
use std::{collections::HashMap, time::Duration};

#[derive(Debug)]
pub struct DeploymentConfig<'a> {
    pub env: Env,
    pub log_level: LogLevel,
    pub address: FlodgattAddr,
    pub port: u16,
    pub unix_socket: Option<Socket>,
    pub cors: Cors<'a>,
    pub sse_interval: SseInterval,
    pub ws_interval: WsInterval,
}
impl Default for DeploymentConfig<'_> {
    fn default() -> Self {
        Self {
            env: Env::Development,
            log_level: LogLevel::Warn,
            address: FlodgattAddr::from_str_or_die(&"127.0.0.1".to_string()),
            port: 4000,
            unix_socket: None,
            cors: Cors {
                allowed_methods: vec!["GET", "OPTIONS"],
                allowed_headers: vec!["Authorization", "Accept", "Cache-Control"],
            },
            sse_interval: SseInterval(Duration::from_millis(100)),
            ws_interval: WsInterval(Duration::from_millis(100)),
        }
    }
}
impl DeploymentConfig<'_> {
    pub fn from_env(env_vars: HashMap<String, String>) -> Self {
        Self::default()
            .maybe_update_env(env_vars.get("NODE_ENV").map(Env::from_str_or_die))
            .maybe_update_env(env_vars.get("RUST_ENV").map(Env::from_str_or_die))
            .maybe_update_address(env_vars.get("BIND").map(FlodgattAddr::from_str_or_die))
            .maybe_update_port(env_vars.get("PORT").map(u16::from_str_or_die))
            .maybe_update_unix_socket(env_vars.get("SOCKET").map(Socket::from_str_or_die))
            .maybe_update_log_level(env_vars.get("RUST_LOG").map(LogLevel::from_str_or_die))
            .maybe_update_sse_interval(env_vars.get("SSE_FREQ").map(SseInterval::from_str_or_die))
            .maybe_update_ws_interval(env_vars.get("WS_FREQ").map(WsInterval::from_str_or_die))
            .log()
    }

    maybe_update!(maybe_update_env; env: Env);
    maybe_update!(maybe_update_port; port: u16);
    maybe_update!(maybe_update_address; address: FlodgattAddr);
    maybe_update!(maybe_update_unix_socket; Some(unix_socket: Socket));
    maybe_update!(maybe_update_log_level; log_level: LogLevel);
    maybe_update!(maybe_update_sse_interval; sse_interval: SseInterval);
    maybe_update!(maybe_update_ws_interval; ws_interval: WsInterval);

    fn log(self) -> Self {
        log::warn!("Using deployment configuration:\n {:#?}", &self);
        self
    }
}
