//! Configuration defaults.  All settings with the prefix of `DEFAULT_` can be overridden
//! by an environmental variable of the same name without that prefix (either by setting
//! the variable at runtime or in the `.env` file)
mod postgres_cfg;
mod redis_cfg;
use self::{postgres_cfg::PostgresConfig, redis_cfg::RedisConfig};
use crate::redis_to_client_stream::redis_cmd;
use dotenv::dotenv;
use lazy_static::lazy_static;
use log::warn;
use std::{env, io::Write, net, time};
use url::Url;

const CORS_ALLOWED_METHODS: [&str; 2] = ["GET", "OPTIONS"];
const CORS_ALLOWED_HEADERS: [&str; 3] = ["Authorization", "Accept", "Cache-Control"];
// Postgres
// Deployment
const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:4000";

const DEFAULT_SSE_UPDATE_INTERVAL: u64 = 100;
const DEFAULT_WS_UPDATE_INTERVAL: u64 = 100;
/// **NOTE**:  Polling Redis is much more time consuming than polling the `Receiver`
///            (on the order of 10ms rather than 50μs).  Thus, changing this setting
///            would be a good place to start for performance improvements at the cost
///            of delaying all updates.
const DEFAULT_REDIS_POLL_INTERVAL: u64 = 100;

lazy_static! {
    pub static ref REDIS_NAMESPACE: Option<String> =  env::var("REDIS_NAMESPACE").ok();

    pub static ref SERVER_ADDR: net::SocketAddr = env::var("SERVER_ADDR")
        .unwrap_or_else(|_| DEFAULT_SERVER_ADDR.to_owned())
        .parse()
        .expect("static string");

    /// Interval, in ms, at which `ClientAgent` polls `Receiver` for updates to send via SSE.
    pub static ref SSE_UPDATE_INTERVAL: u64 = env::var("SSE_UPDATE_INTERVAL")
        .map(|s| s.parse().expect("Valid config"))
        .unwrap_or(DEFAULT_SSE_UPDATE_INTERVAL);
    /// Interval, in ms, at which `ClientAgent` polls `Receiver` for updates to send via WS.
    pub static ref WS_UPDATE_INTERVAL: u64 = env::var("WS_UPDATE_INTERVAL")
        .map(|s| s.parse().expect("Valid config"))
        .unwrap_or(DEFAULT_WS_UPDATE_INTERVAL);
    /// Interval, in ms, at which the `Receiver` polls Redis.
    pub static ref REDIS_POLL_INTERVAL: u64 = env::var("REDIS_POLL_INTERVAL")
            .map(|s| s.parse().expect("Valid config"))
            .unwrap_or(DEFAULT_REDIS_POLL_INTERVAL);
}

/// Configure CORS for the API server
pub fn cross_origin_resource_sharing() -> warp::filters::cors::Cors {
    warp::cors()
        .allow_any_origin()
        .allow_methods(CORS_ALLOWED_METHODS.to_vec())
        .allow_headers(CORS_ALLOWED_HEADERS.to_vec())
}

/// Initialize logging and read values from `src/.env`
pub fn logging_and_env() {
    dotenv().ok();
    pretty_env_logger::init();
}

/// Configure Postgres and return a connection
pub fn postgres() -> postgres::Client {
    // use openssl::ssl::{SslConnector, SslMethod};
    // use postgres_openssl::MakeTlsConnector;
    // let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    // builder.set_ca_file("/etc/ssl/cert.pem").unwrap();
    // let connector = MakeTlsConnector::new(builder.build());
    // TODO: add TLS support, remove `NoTls`
    let pg_cfg = match &env::var("DATABASE_URL").ok() {
        Some(url) => {
            warn!("DATABASE_URL env variable set.  Connecting to Postgres with that URL and ignoring any values set in DB_HOST, DB_USER, DB_NAME, DB_PASS, or DB_PORT.");
            PostgresConfig::from_url(Url::parse(url).unwrap())
        }
        None => PostgresConfig::default()
            .maybe_update_user(env::var("USER").ok())
            .maybe_update_user(env::var("DB_USER").ok())
            .maybe_update_host(env::var("DB_HOST").ok())
            .maybe_update_pass(env::var("DB_PASS").ok())
            .maybe_update_db(env::var("DB_NAME").ok())
            .maybe_update_sslmode(env::var("DB_SSLMODE").ok()),
    };
    log::warn!(
        "Connecting to Postgres with the following configuration:\n{:#?}",
        &pg_cfg
    );

    let mut con = postgres::Client::configure();
    con.user(&pg_cfg.user)
        .host(&pg_cfg.host)
        .port(pg_cfg.port.parse::<u16>().unwrap())
        .dbname(&pg_cfg.database);
    if let Some(password) = &pg_cfg.password {
        con.password(password);
    };
    con.connect(postgres::NoTls)
        .expect("Can connect to local Postgres")
}

/// Configure Redis and return a pair of connections
pub fn redis_addr() -> (net::TcpStream, net::TcpStream) {
    let redis_cfg = match &env::var("REDIS_URL") {
        Ok(url) => {
            warn!("REDIS_URL env variable set.  Connecting to Redis with that URL and ignoring any values set in REDIS_HOST or DB_PORT.");
            RedisConfig::from_url(Url::parse(url).unwrap())
        }
        Err(_) => RedisConfig::default()
            .maybe_update_host(env::var("REDIS_HOST").ok())
            .maybe_update_port(env::var("REDIS_PORT").ok()),
    };
    log::warn!(
        "Connecting to Redis with the following configuration:\n{:#?}",
        &redis_cfg
    );

    let addr = format!("{}:{}", redis_cfg.host, redis_cfg.port);
    if let Some(user) = &redis_cfg.user {
        log::error!(
            "Username {} provided, but Redis does not need a username.  Ignoring it",
            user
        );
    };
    let mut pubsub_connection =
        net::TcpStream::connect(addr.clone()).expect("Can connect to Redis");
    pubsub_connection
        .set_read_timeout(Some(time::Duration::from_millis(10)))
        .expect("Can set read timeout for Redis connection");
    pubsub_connection
        .set_nonblocking(true)
        .expect("set_nonblocking call failed");
    let mut secondary_redis_connection =
        net::TcpStream::connect(addr).expect("Can connect to Redis");
    secondary_redis_connection
        .set_read_timeout(Some(time::Duration::from_millis(10)))
        .expect("Can set read timeout for Redis connection");
    if let Some(password) = redis_cfg.password {
        pubsub_connection
            .write_all(&redis_cmd::cmd("auth", &password))
            .unwrap();
        secondary_redis_connection
            .write_all(&redis_cmd::cmd("auth", password))
            .unwrap();
    }

    if let Some(db) = redis_cfg.db {
        pubsub_connection
            .write_all(&redis_cmd::cmd("SELECT", &db))
            .unwrap();
        secondary_redis_connection
            .write_all(&redis_cmd::cmd("SELECT", &db))
            .unwrap();
    }

    (pubsub_connection, secondary_redis_connection)
}
