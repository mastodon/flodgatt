use super::redis_cfg_types::*;
use crate::config::EnvVar;

#[derive(Debug, Default)]
pub struct RedisConfig {
    pub user: RedisUser,
    pub password: RedisPass,
    pub port: RedisPort,
    pub host: RedisHost,
    pub db: RedisDb,
    pub namespace: RedisNamespace,
    // **NOTE**:  Polling Redis is much more time consuming than polling the `Receiver` (~1ms
    // compared to ~50μs).  Thus, changing this setting with REDIS_POLL_INTERVAL may be a good
    // place to start for performance improvements at the cost of delaying all updates.
    pub polling_interval: RedisInterval,
}

impl RedisConfig {
    const USER_SET_WARNING: &'static str =
        "Redis user specified, but Redis did not ask for a username.  Ignoring it.";
    const DB_SET_WARNING: &'static str =
        r"Redis database specified, but PubSub connections do not use databases.
For similar functionality, you may wish to set a REDIS_NAMESPACE";

    pub fn from_env(env: EnvVar) -> Self {
        let env = match env.get("REDIS_URL").cloned() {
            Some(url_str) => env.update_with_url(&url_str),
            None => env,
        };

        let cfg = RedisConfig {
            user: RedisUser::default().maybe_update(env.get("REDIS_USER")),
            password: RedisPass::default().maybe_update(env.get("REDIS_PASSWORD")),
            port: RedisPort::default().maybe_update(env.get("REDIS_PORT")),
            host: RedisHost::default().maybe_update(env.get("REDIS_HOST")),
            db: RedisDb::default().maybe_update(env.get("REDIS_DB")),
            namespace: RedisNamespace::default().maybe_update(env.get("REDIS_NAMESPACE")),
            polling_interval: RedisInterval::default().maybe_update(env.get("REDIS_POLL_INTERVAL")),
        };

        if cfg.db.is_some() {
            log::warn!("{}", Self::DB_SET_WARNING);
        }
        if cfg.user.is_some() {
            log::warn!("{}", Self::USER_SET_WARNING);
        }
        log::info!("Redis configuration:\n{:#?},", &cfg);
        cfg
    }
}
