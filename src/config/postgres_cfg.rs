use crate::{err, maybe_update};
use url::Url;

#[derive(Debug)]
pub struct PostgresConfig {
    pub user: String,
    pub host: String,
    pub password: Option<String>,
    pub database: String,
    pub port: String,
    pub ssl_mode: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            user: "postgres".to_string(),
            host: "localhost".to_string(),
            password: None,
            database: "mastodon_development".to_string(),
            port: "5432".to_string(),
            ssl_mode: "prefer".to_string(),
        }
    }
}
fn none_if_empty(item: &str) -> Option<String> {
    Some(item).filter(|i| !i.is_empty()).map(String::from)
}

impl PostgresConfig {
    maybe_update!(maybe_update_user; user);
    maybe_update!(maybe_update_host; host);
    maybe_update!(maybe_update_db; database);
    maybe_update!(maybe_update_port; port);
    maybe_update!(maybe_update_sslmode; ssl_mode);
    maybe_update!(maybe_update_password; Some(password));

    pub fn from_url(url: Url) -> Self {
        let (mut user, mut host, mut sslmode, mut password) = (None, None, None, None);
        for (k, v) in url.query_pairs() {
            match k.to_string().as_str() {
                "user" => { user = Some(v.to_string());},
                "password" => { password = Some(v.to_string());},
                "host" => { host = Some(v.to_string());},
                "sslmode" => { sslmode = Some(v.to_string());},
                _ => { err::die_with_msg(format!("Unsupported parameter {} in DATABASE_URL.\n   Flodgatt supports only `user`, `password`, `host`, and `sslmode` parameters.", k))}
            }
        }

        Self::default()
            // Values from query parameter
            .maybe_update_user(user)
            .maybe_update_password(password)
            .maybe_update_host(host)
            .maybe_update_sslmode(sslmode)
            // Values from URL (which override query values if both are present)
            .maybe_update_user(none_if_empty(url.username()))
            .maybe_update_host(url.host_str().filter(|h| !h.is_empty()).map(String::from))
            .maybe_update_password(url.password().map(String::from))
            .maybe_update_port(url.port().map(|port_num| port_num.to_string()))
            .maybe_update_db(none_if_empty(&url.path()[1..]))
    }
}
