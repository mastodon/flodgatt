//! Configuration defaults.  All settings with the prefix of `DEFAULT_` can be overridden
//! by an environmental variable of the same name without that prefix (either by setting
//! the variable at runtime or in the `.env` file)
use dotenv::dotenv;
use lazy_static::lazy_static;
use log::warn;
use std::{env, io::Write, net, time};
use url::Url;

use crate::{err, redis_to_client_stream::redis_cmd};

const CORS_ALLOWED_METHODS: [&str; 2] = ["GET", "OPTIONS"];
const CORS_ALLOWED_HEADERS: [&str; 3] = ["Authorization", "Accept", "Cache-Control"];
// Postgres
const DEFAULT_DB_HOST: &str = "localhost";
const DEFAULT_DB_USER: &str = "postgres";
const DEFAULT_DB_NAME: &str = "mastodon_development";
const DEFAULT_DB_PORT: &str = "5432";
const DEFAULT_DB_SSLMODE: &str = "prefer";
// Redis
const DEFAULT_REDIS_HOST: &str = "127.0.0.1";
const DEFAULT_REDIS_PORT: &str = "6379";

const _DEFAULT_REDIS_NAMESPACE: &str = "";
// Deployment
const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:4000";

const DEFAULT_SSE_UPDATE_INTERVAL: u64 = 100;
const DEFAULT_WS_UPDATE_INTERVAL: u64 = 100;
/// **NOTE**:  Polling Redis is much more time consuming than polling the `Receiver`
///            (on the order of 10ms rather than 50μs).  Thus, changing this setting
///            would be a good place to start for performance improvements at the cost
///            of delaying all updates.
const DEFAULT_REDIS_POLL_INTERVAL: u64 = 100;

fn default(var: &str, default_var: &str) -> String {
    env::var(var)
        .unwrap_or_else(|_| {
            warn!(
                "No {} env variable set. Using default value: {}",
                var, default_var
            );
            default_var.to_string()
        })
        .to_string()
}

lazy_static! {
    static ref POSTGRES_ADDR: String = match &env::var("DATABASE_URL") {
        Ok(url) => {
            warn!("DATABASE_URL env variable set.  Connecting to Postgres with that URL and ignoring any values set in DB_HOST, DB_USER, DB_NAME, DB_PASS, or DB_PORT.");
            url.to_string()
        }
        Err(_) => {
            let user = &env::var("DB_USER").unwrap_or_else(|_| {
                match &env::var("USER") {
                    Err(_) => default("DB_USER", DEFAULT_DB_USER),
                    Ok(user) => default("DB_USER", user)
                }
            });
            let host = &env::var("DB_HOST")
                .unwrap_or_else(|_| default("DB_HOST", DEFAULT_DB_HOST));
            let db_name = &env::var("DB_NAME")
                .unwrap_or_else(|_| default("DB_NAME", DEFAULT_DB_NAME));
            let port = &env::var("DB_PORT")
                .unwrap_or_else(|_| default("DB_PORT", DEFAULT_DB_PORT));
            let ssl_mode = &env::var("DB_SSLMODE")
                .unwrap_or_else(|_| default("DB_SSLMODE", DEFAULT_DB_SSLMODE));


            match &env::var("DB_PASS") {
                Ok(password) => {
                    format!("postgres://{}:{}@{}:{}/{}?sslmode={}",
                            user, password, host, port, db_name, ssl_mode)},
                Err(_) => {
                    warn!("No DB_PASSWORD set.  Attempting to connect to Postgres without a password.  (This is correct if you are using the `ident` method.)");
                    format!("postgres://{}@{}:{}/{}?sslmode={}",
                            user, host, port, db_name, ssl_mode)
                },
            }
        }
    };
    static ref REDIS_ADDR: RedisConfig = match &env::var("REDIS_URL") {
        Ok(url) => {
            warn!(r"REDIS_URL env variable set.
    Connecting to Redis with that URL and ignoring any values set in REDIS_HOST or DB_PORT.");
            let url = Url::parse(url).unwrap();
            fn none_if_empty(item: &str) -> Option<String> {
                if item.is_empty() { None } else { Some(item.to_string()) }
            };


            let user = none_if_empty(url.username());
            let mut password = url.password().as_ref().map(|str| str.to_string());
            let host = err::unwrap_or_die(url.host_str(),"Missing/invalid host in REDIS_URL");
            let port = err::unwrap_or_die(url.port(), "Missing/invalid port in REDIS_URL");
            let mut db = none_if_empty(url.path());
            let query_pairs = url.query_pairs();

            for (key, value) in query_pairs {
                match key.to_string().as_str() {
                    "password" => { password = Some(value.to_string());},
                    "db" => { db = Some(value.to_string())}
                    _ => { err::die_with_msg(format!("Unsupported parameter {} in REDIS_URL.\n   Flodgatt supports only `password` and `db` parameters.", key))}
                }
            }
            RedisConfig {
                user,
                password,
                host,
                port,
                db
            }
        }
        Err(_) => {
            let host = env::var("REDIS_HOST")
                .unwrap_or_else(|_| default("REDIS_HOST", DEFAULT_REDIS_HOST));
            let port = env::var("REDIS_PORT")
                .unwrap_or_else(|_| default("REDIS_PORT", DEFAULT_REDIS_PORT));
            RedisConfig {
                user: None,
                password: None,
                host,
                port,
                db: None,
            }
        }
    };


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
    POSTGRES_ADDR.to_string();
}

/// Configure Postgres and return a connection
pub fn postgres() -> postgres::Client {
    // use openssl::ssl::{SslConnector, SslMethod};
    // use postgres_openssl::MakeTlsConnector;
    // let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    // builder.set_ca_file("/etc/ssl/cert.pem").unwrap();
    // let connector = MakeTlsConnector::new(builder.build());
    // TODO: add TLS support, remove `NoTls`
    postgres::Client::connect(&POSTGRES_ADDR.to_string(), postgres::NoTls)
        .expect("Can connect to local Postgres")
}
#[derive(Default)]
struct RedisConfig {
    user: Option<String>,
    password: Option<String>,
    port: String,
    host: String,
    db: Option<String>,
}
/// Configure Redis
pub fn redis_addr() -> (net::TcpStream, net::TcpStream) {
    let redis = &REDIS_ADDR;
    let addr = format!("{}:{}", redis.host, redis.port);
    if let Some(user) = &redis.user {
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
    if let Some(password) = &REDIS_ADDR.password {
        pubsub_connection
            .write_all(&redis_cmd::cmd("auth", &password))
            .unwrap();
        secondary_redis_connection
            .write_all(&redis_cmd::cmd("auth", password))
            .unwrap();
    } else {
        warn!("No REDIS_PASSWORD set.  Attempting to connect to Redis without a password.  (This is correct if you are following the default setup.)");
    }
    if let Some(db) = &REDIS_ADDR.db {
        pubsub_connection
            .write_all(&redis_cmd::cmd("SELECT", &db))
            .unwrap();
        secondary_redis_connection
            .write_all(&redis_cmd::cmd("SELECT", &db))
            .unwrap();
    }
    (pubsub_connection, secondary_redis_connection)
}
