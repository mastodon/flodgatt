use super::{deployment_cfg_types::*, EnvVar};
use crate::err::FatalErr;

#[derive(Debug, Default)]
pub struct Deployment<'a> {
    pub(crate) env: Env,
    pub(crate) log_level: LogLevel,
    pub address: FlodgattAddr,
    pub port: Port,
    pub unix_socket: Socket,
    pub cors: Cors<'a>,
    pub whitelist_mode: WhitelistMode,
}

impl Deployment<'_> {
    pub(crate) fn from_env(env: &EnvVar) -> Result<Self, FatalErr> {
        let mut cfg = Self {
            env: Env::default().maybe_update(env.get("NODE_ENV"))?,
            log_level: LogLevel::default().maybe_update(env.get("RUST_LOG"))?,
            address: FlodgattAddr::default().maybe_update(env.get("BIND"))?,
            port: Port::default().maybe_update(env.get("PORT"))?,
            unix_socket: Socket::default().maybe_update(env.get("SOCKET"))?,
            whitelist_mode: WhitelistMode::default().maybe_update(env.get("WHITELIST_MODE"))?,
            cors: Cors::default(),
        };
        cfg.env = cfg.env.maybe_update(env.get("RUST_ENV"))?;
        Ok(cfg)
    }
}
