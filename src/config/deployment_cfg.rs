use crate::config::deployment_cfg_types::*;
use std::collections::HashMap;

#[derive(Debug, Default)]
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

impl DeploymentConfig<'_> {
    pub fn from_env(env_vars: HashMap<String, String>) -> Self {
        let mut res = Self::default();
        res.env = Env::from_env_var_or_die(env_vars.get("NODE_ENV"));
        res.env = Env::from_env_var_or_die(env_vars.get("RUST_ENV"));
        res.log_level = LogLevel::from_env_var_or_die(env_vars.get("RUST_LOG"));
        res.address = FlodgattAddr::from_env_var_or_die(env_vars.get("BIND"));
        res.port = Port2::from_env_var_or_die(env_vars.get("PORT"));
        res.unix_socket = Socket::from_env_var_or_die(env_vars.get("SOCKET"));
        res.sse_interval = SseInterval::from_env_var_or_die(env_vars.get("SSE_FREQ"));
        res.ws_interval = WsInterval::from_env_var_or_die(env_vars.get("WS_FREQ"));

        res.log()
    }

    fn log(self) -> Self {
        log::warn!("Using deployment configuration:\n {:#?}", &self);
        self
    }
}
