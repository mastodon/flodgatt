use crate::{err, maybe_update};
use std::collections::HashMap;
use url::Url;

#[derive(Debug)]
pub struct PostgresConfig {
    pub user: String,
    pub host: String,
    pub password: Option<String>,
    pub database: String,
    pub port: u16,
    pub ssl_mode: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            user: "postgres".to_string(),
            host: "localhost".to_string(),
            password: None,
            database: "mastodon_development".to_string(),
            port: 5432,
            ssl_mode: "prefer".to_string(),
        }
    }
}
fn none_if_empty(item: &str) -> Option<String> {
    Some(item).filter(|i| !i.is_empty()).map(String::from)
}

impl PostgresConfig {
    /// Configure Postgres and return a connection
    pub fn from_env(env_vars: HashMap<String, String>) -> Self {
        // use openssl::ssl::{SslConnector, SslMethod};
        // use postgres_openssl::MakeTlsConnector;
        // let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        // builder.set_ca_file("/etc/ssl/cert.pem").unwrap();
        // let connector = MakeTlsConnector::new(builder.build());
        // TODO: add TLS support, remove `NoTls`
        match env_vars.get("DATABASE_URL") {
            Some(url) => {
            log::warn!("DATABASE_URL env variable set.  Connecting to Postgres with that URL and ignoring any values set in DB_HOST, DB_USER, DB_NAME, DB_PASS, or DB_PORT.");
            PostgresConfig::from_url(Url::parse(url).unwrap())
            }
            None => Self::default()
                .maybe_update_user(env_vars.get("USER").map(String::from))
                .maybe_update_user(env_vars.get("DB_USER").map(String::from))
                .maybe_update_host(env_vars.get("DB_HOST").map(String::from))
                .maybe_update_password(env_vars.get("DB_PASS").map(String::from))
                .maybe_update_db(env_vars.get("DB_NAME").map(String::from))
                .maybe_update_sslmode(env_vars.get("DB_SSLMODE").map(String::from))}
        .log()
    }
    maybe_update!(maybe_update_user; user: String);
    maybe_update!(maybe_update_host; host: String);
    maybe_update!(maybe_update_db; database: String);
    maybe_update!(maybe_update_port; port: u16);
    maybe_update!(maybe_update_sslmode; ssl_mode: String);
    maybe_update!(maybe_update_password; Some(password: String));

    fn from_url(url: Url) -> Self {
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
            .maybe_update_port(url.port())
            .maybe_update_db(none_if_empty(&url.path()[1..]))
    }
    fn log(self) -> Self {
        log::warn!("Postgres configuration:\n{:#?}", &self);
        self
    }
}
