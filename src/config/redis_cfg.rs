use crate::{err, maybe_update};
use std::{collections::HashMap, time::Duration};
use url::Url;

fn none_if_empty(item: &str) -> Option<String> {
    Some(item).filter(|i| !i.is_empty()).map(String::from)
}

#[derive(Debug)]
pub struct RedisConfig {
    pub user: Option<String>,
    pub password: Option<String>,
    pub port: u16,
    pub host: String,
    pub db: Option<String>,
    pub namespace: Option<String>,
    // **NOTE**:  Polling Redis is much more time consuming than polling the `Receiver`
    //            (on the order of 1ms rather than 50μs).  Thus, changing this setting
    //            would be a good place to start for performance improvements at the cost
    //            of delaying all updates.
    pub polling_interval: Duration,
}
impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            user: None,
            password: None,
            db: None,
            port: 6379,
            host: "127.0.0.1".to_string(),
            namespace: None,
            polling_interval: Duration::from_millis(100),
        }
    }
}
impl RedisConfig {
    pub fn from_env(env_vars: HashMap<String, String>) -> Self {
        match env_vars.get("REDIS_URL") {
            Some(url) => {
                log::warn!("REDIS_URL env variable set.  Connecting to Redis with that URL and ignoring any values set in REDIS_HOST or DB_PORT.");
                Self::from_url(Url::parse(url).unwrap())
            }
            None => RedisConfig::default()
                .maybe_update_host(env_vars.get("REDIS_HOST").map(String::from))
                .maybe_update_port(env_vars.get("REDIS_PORT").map(|p| err::unwrap_or_die(
                    p.parse().ok(),"REDIS_PORT must be a number."))),
        }
        .maybe_update_namespace(env_vars.get("REDIS_NAMESPACE").map(String::from))
        .maybe_update_polling_interval(env_vars.get("REDIS_POLL_INTERVAL")
            .map(|str| Duration::from_millis(str.parse().unwrap()))).log()
    }

    fn from_url(url: Url) -> Self {
        let mut password = url.password().as_ref().map(|str| str.to_string());
        let mut db = none_if_empty(&url.path()[1..]);
        for (k, v) in url.query_pairs() {
            match k.to_string().as_str() {
                "password" => { password = Some(v.to_string());},
                "db" => { db = Some(v.to_string())},
                _ => { err::die_with_msg(format!("Unsupported parameter {} in REDIS_URL.\n   Flodgatt supports only `password` and `db` parameters.", k))}
                }
        }
        let user = none_if_empty(url.username());
        if let Some(user) = &user {
            log::error!(
                "Username {} provided, but Redis does not need a username.  Ignoring it",
                user
            );
        }
        RedisConfig {
            user,
            host: err::unwrap_or_die(url.host_str(), "Missing or invalid host in REDIS_URL")
                .to_string(),
            port: err::unwrap_or_die(url.port(), "Missing or invalid port in REDIS_URL"),
            namespace: None,
            password,
            db,
            polling_interval: Duration::from_millis(100),
        }
    }

    maybe_update!(maybe_update_host; host: String);
    maybe_update!(maybe_update_port; port: u16);
    maybe_update!(maybe_update_namespace; Some(namespace: String));
    maybe_update!(maybe_update_polling_interval; polling_interval: Duration);

    fn log(self) -> Self {
        log::warn!("Redis configuration:\n{:#?},", &self);
        self
    }
}
