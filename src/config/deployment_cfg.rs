use crate::{config::deployment_cfg_types::*, err::FromStrOrDie, maybe_update};
use std::collections::HashMap;

#[derive(Debug)]
pub struct DeploymentConfig<'a> {
    pub env: Env,
    pub log_level: LogLevel,
    pub address: FlodgattAddr,
    pub port: Port2,
    pub unix_socket: Socket,
    pub cors: Cors<'a>,
    pub sse_interval: SseInterval,
    pub ws_interval: WsInterval,
}
impl Default for DeploymentConfig<'_> {
    fn default() -> Self {
        Self {
            env: Env::Development,
            log_level: LogLevel::default(),
            address: FlodgattAddr::default(),
            port: Port2::default(),
            unix_socket: Socket::default(),
            cors: Cors {
                allowed_methods: vec!["GET", "OPTIONS"],
                allowed_headers: vec!["Authorization", "Accept", "Cache-Control"],
            },
            sse_interval: SseInterval::default(),
            ws_interval: WsInterval::default(),
        }
    }
}
impl DeploymentConfig<'_> {
    pub fn from_env(env_vars: HashMap<String, String>) -> Self {
        let mut res = Self::default()
            .maybe_update_env(env_vars.get("NODE_ENV").map(Env::from_str_or_die))
            .maybe_update_env(env_vars.get("RUST_ENV").map(Env::from_str_or_die));

        res.log_level = LogLevel::from_env_var_or_die(env_vars.get("RUST_LOG"));
        res.address = FlodgattAddr::from_env_var_or_die(env_vars.get("BIND"));
        res.port = Port2::from_env_var_or_die(env_vars.get("PORT"));
        res.unix_socket = Socket::from_env_var_or_die(env_vars.get("SOCKET"));
        res.sse_interval = SseInterval::from_env_var_or_die(env_vars.get("SSE_FREQ"));
        res.ws_interval = WsInterval::from_env_var_or_die(env_vars.get("WS_FREQ"));

        res.log()
    }

    maybe_update!(maybe_update_env; env: Env);
    //    maybe_update!(maybe_update_port; port: Port);
    maybe_update!(maybe_update_address; address: FlodgattAddr);
    //    maybe_update!(maybe_update_unix_socket; Some(unix_socket: Socket));
    //    maybe_update!(maybe_update_log_level; log_level: LogLevel);
    //    maybe_update!(maybe_update_sse_int; sse_interval: SseInterval);
    //    maybe_update!(maybe_update_ws_int; ws_interval: WsInterval);

    fn log(self) -> Self {
        log::warn!("Using deployment configuration:\n {:#?}", &self);
        self
    }
}
