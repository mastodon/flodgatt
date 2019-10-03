use crate::err;
use url::Url;

fn none_if_empty(item: &str) -> Option<String> {
    if item.is_empty() {
        None
    } else {
        Some(item.to_string())
    }
}

#[derive(Debug)]
pub struct RedisConfig {
    pub user: Option<String>,
    pub password: Option<String>,
    pub port: String,
    pub host: String,
    pub db: Option<String>,
}
impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            user: None,
            password: None,
            db: None,
            port: "6379".to_string(),
            host: "127.0.0.1".to_string(),
        }
    }
}
impl RedisConfig {
    pub fn from_url(url: Url) -> Self {
        let mut password = url.password().as_ref().map(|str| str.to_string());
        let mut db = none_if_empty(&url.path()[1..]);
        for (k, v) in url.query_pairs() {
            match k.to_string().as_str() {
                "password" => { password = Some(v.to_string());},
                "db" => { db = Some(v.to_string())},
                _ => { err::die_with_msg(format!("Unsupported parameter {} in REDIS_URL.\n   Flodgatt supports only `password` and `db` parameters.", k))}
                }
        }
        RedisConfig {
            user: none_if_empty(url.username()),
            host: err::unwrap_or_die(url.host_str(), "Missing or invalid host in REDIS_URL"),
            port: err::unwrap_or_die(url.port(), "Missing or invalid port in REDIS_URL"),
            password,
            db,
        }
    }
    pub fn maybe_update_host(self, host: Option<String>) -> Self {
        match host {
            Some(host) => Self { host, ..self },
            _ => Self { ..self },
        }
    }
    pub fn maybe_update_port(self, port: Option<String>) -> Self {
        match port {
            Some(port) => Self { port, ..self },
            _ => Self { ..self },
        }
    }
}
