use super::redis_cfg_types::*;
//use crate::{err, maybe_update};
use crate::maybe_update;
use std::collections::HashMap;
//use url::Url;

fn none_if_empty(item: &str) -> Option<String> {
    Some(item).filter(|i| !i.is_empty()).map(String::from)
}

#[derive(Debug)]
pub struct RedisConfig {
    pub user: Option<String>,
    pub password: RedisPass,
    pub port: RedisPort,
    pub host: RedisHost,
    pub db: Option<String>,
    pub namespace: Option<String>,
    // **NOTE**:  Polling Redis is much more time consuming than polling the `Receiver`
    //            (on the order of 1ms rather than 50μs).  Thus, changing this setting
    //            would be a good place to start for performance improvements at the cost
    //            of delaying all updates.
    pub polling_interval: RedisInterval,
}
impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            user: None,
            password: RedisPass::default(),
            db: None,
            port: RedisPort::default(),
            host: RedisHost::default(),
            namespace: None,
            polling_interval: RedisInterval::default(),
        }
    }
}

impl RedisConfig {
    pub fn from_env(env_vars: HashMap<String, String>) -> Self {
        // TODO handle REDIS_URL

        let mut cfg = RedisConfig::default();
        cfg.host = RedisHost::default().maybe_update(env_vars.get("REDIS_HOST"));
        cfg = cfg.maybe_update_namespace(env_vars.get("REDIS_NAMESPACE").map(String::from));

        cfg.port = RedisPort::default().maybe_update(env_vars.get("REDIS_PORT"));
        cfg.polling_interval =
            RedisInterval::default().maybe_update(env_vars.get("REDIS_POLL_INTERVAL"));
        cfg.password = RedisPass::default().maybe_update(env_vars.get("REDIS_PASSWORD"));

        cfg.log()
    }

    //    maybe_update!(maybe_update_host; host: String);
    //    maybe_update!(maybe_update_port; port: u16);
    maybe_update!(maybe_update_namespace; Some(namespace: String));
    //    maybe_update!(maybe_update_polling_interval; polling_interval: Duration);

    fn log(self) -> Self {
        log::warn!("Redis configuration:\n{:#?},", &self);
        self
    }
}
