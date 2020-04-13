use super::{deployment_cfg_types::*, EnvVar};

#[derive(Debug, Default)]
pub struct Deployment<'a> {
    pub env: Env,
    pub log_level: LogLevel,
    pub address: FlodgattAddr,
    pub port: Port,
    pub unix_socket: Socket,
    pub cors: Cors<'a>,
    pub sse_interval: SseInterval,
    pub ws_interval: WsInterval,
    pub whitelist_mode: WhitelistMode,
}

impl Deployment<'_> {
    pub fn from_env(env: EnvVar) -> Self {
        let mut cfg = Self {
            env: Env::default().maybe_update(env.get("NODE_ENV")),
            log_level: LogLevel::default().maybe_update(env.get("RUST_LOG")),
            address: FlodgattAddr::default().maybe_update(env.get("BIND")),
            port: Port::default().maybe_update(env.get("PORT")),
            unix_socket: Socket::default().maybe_update(env.get("SOCKET")),
            sse_interval: SseInterval::default().maybe_update(env.get("SSE_FREQ")),
            ws_interval: WsInterval::default().maybe_update(env.get("WS_FREQ")),
            whitelist_mode: WhitelistMode::default().maybe_update(env.get("WHITELIST_MODE")),
            cors: Cors::default(),
        };
        cfg.env = cfg.env.maybe_update(env.get("RUST_ENV"));
        log::info!("Using deployment configuration:\n {:#?}", &cfg);
        cfg
    }
}
