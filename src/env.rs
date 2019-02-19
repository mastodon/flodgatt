use envconfig::Envconfig;
use std::net::IpAddr;

/// Returns the current users username.
/// TODO: Find a way to do this cross-platform
pub fn current_user() -> String {
    whoami::username()
}

#[cfg(feature = "production")]
#[derive(Envconfig)]
/// Production DB configuration
pub struct DbConfig {
    #[envconfig(from = "DB_USER", default = "mastodon")]
    user: String,
    #[envconfig(from = "DB_PASS", default = "")]
    password: String,
    #[envconfig(from = "DB_NAME", default = "mastodon_production")]
    database: String,
    #[envconfig(from = "DB_HOST", default = "localhost")]
    host: String,
    #[envconfig(from = "DB_PORT", default = "5432")]
    port: u16,
}

#[cfg(not(feature = "production"))]
#[derive(Envconfig)]
/// Development DB configuration
pub struct DbConfig {
    #[envconfig(from = "DB_USER", default = current_user())]
    pub user: String,
    #[envconfig(from = "DB_PASS", default = "")]
    pub password: String,
    #[envconfig(from = "DB_NAME", default = "mastodon_development")]
    pub database: String,
    #[envconfig(from = "DB_HOST", default = "localhost")]
    pub host: String,
    #[envconfig(from = "DB_PORT", default = "5432")]
    pub port: u16,
}

#[derive(Envconfig)]
pub struct ServerConfig {
    #[envconfig(from = "BIND", default = "0.0.0.0")]
    pub address: IpAddr,
    #[envconfig(from = "PORT", default = "4000")]
    pub port: u16,
}

#[derive(Envconfig)]
pub struct RedisConfig {
    #[envconfig(from = "REDIS_HOST", default = "127.0.0.1")]
    pub host: IpAddr,
    #[envconfig(from = "REDIS_PORT", default = "6379")]
    pub port: u16,
    #[envconfig(from = "REDIS_DB", default = "0")]
    pub db: u16,
    #[envconfig(from = "REDIS_PASSWORD", default = "")]
    pub password: String,
}
