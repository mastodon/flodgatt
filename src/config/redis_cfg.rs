use super::redis_cfg_types::*;
use crate::config::EnvVar;
use url::Url;

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

impl EnvVar {
    fn update_with_redis_url(mut self, url_str: &str) -> Self {
        let url = Url::parse(url_str).unwrap();
        let none_if_empty = |s: String| if s.is_empty() { None } else { Some(s) };

        self.maybe_add_env_var("REDIS_PORT", url.port());
        self.maybe_add_env_var("REDIS_PASSWORD", url.password());
        self.maybe_add_env_var("REDIS_USERNAME", none_if_empty(url.username().to_string()));
        self.maybe_add_env_var("REDIS_DB", none_if_empty(url.path()[1..].to_string()));
        for (k, v) in url.query_pairs().into_owned() {
            match k.to_string().as_str() {
                "password" => self.maybe_add_env_var("REDIS_PASSWORD", Some(v.to_string())),
                "db" => self.maybe_add_env_var("REDIS_DB", Some(v.to_string())),
                _ => crate::err::die_with_msg(format!(
                    r"Unsupported parameter {} in REDIS_URL.
             Flodgatt supports only `password` and `db` parameters.",
                    k
                )),
            }
        }
        self
    }
}

impl RedisConfig {
    const USER_SET_WARNING: &'static str =
        "Redis user specified, but Redis did not ask for a username.  Ignoring it.";
    const DB_SET_WARNING: &'static str =
        r"Redis database specified, but PubSub connections do not use databases.
For similar functionality, you may wish to set a REDIS_NAMESPACE";

    pub fn from_env(env: EnvVar) -> Self {
        let env = match env.get("REDIS_URL").cloned() {
            Some(url_str) => env.update_with_redis_url(&url_str),
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
        log::warn!("Redis configuration:\n{:#?},", &cfg);
        cfg
    }
}
