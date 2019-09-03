//! Configuration defaults.  All settings with the prefix of `DEFAULT_` can be overridden
//! by an environmental variable of the same name without that prefix (either by setting
//! the variable at runtime or in the `.env` file)
use dotenv::dotenv;
use lazy_static::lazy_static;
use log::warn;
use serde_derive::Serialize;
use std::{env, net, time};

const CORS_ALLOWED_METHODS: [&str; 2] = ["GET", "OPTIONS"];
const CORS_ALLOWED_HEADERS: [&str; 3] = ["Authorization", "Accept", "Cache-Control"];
// Postgres
const DEFAULT_DB_HOST: &str = "localhost";
const DEFAULT_DB_USER: &str = "postgres";
const DEFAULT_DB_NAME: &str = "mastodon_development";
const DEFAULT_DB_PORT: &str = "5432";
const DEFAULT_DB_SSLMODE: &str = "prefer";
// Redis
const DEFAULT_REDIS_ADDR: &str = "127.0.0.1:6379";
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
                "No {} env variable set for Postgres. Using default value: {}",
                var, default_var
            );
            default_var.to_string()
        })
        .to_string()
}

lazy_static! {

    static ref POSTGRES_ADDR: String = match &env::var("POSTGRESS_ADDR") {
        Ok(url) => {
            warn!("DATABASE_URL env variable set.  Trying to connect to Postgres with that URL instead of any values set in DB_HOST, DB_USER, DB_NAME, DB_PASS, or DB_PORT.");
            url.to_string()
        }
        Err(_) => {
            let user = &env::var("DB_USER").unwrap_or_else(|_| {
                match &env::var("USER") {
                    Err(_) => default("DB_USER", DEFAULT_DB_USER),
                    Ok(user) => default("DB_USER", user)
                }
            });
            let host = &env::var("DB_HOST").unwrap_or(default("DB_HOST", DEFAULT_DB_HOST));
          //      .unwrap_or_else(|_| default("DB_HOST", DEFAULT_DB_HOST));
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
    static ref REDIS_ADDR: String = env::var("REDIS_ADDR")
        .unwrap_or_else(|_| DEFAULT_REDIS_ADDR.to_owned());

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
    postgres::Client::connect(&POSTGRES_ADDR.to_string(), postgres::NoTls)
        .expect("Can connect to local Postgres")
}

/// Configure Redis
pub fn redis_addr() -> (net::TcpStream, net::TcpStream) {
    let pubsub_connection =
        net::TcpStream::connect(&REDIS_ADDR.to_string()).expect("Can connect to Redis");
    pubsub_connection
        .set_read_timeout(Some(time::Duration::from_millis(10)))
        .expect("Can set read timeout for Redis connection");
    let secondary_redis_connection =
        net::TcpStream::connect(&REDIS_ADDR.to_string()).expect("Can connect to Redis");
    secondary_redis_connection
        .set_read_timeout(Some(time::Duration::from_millis(10)))
        .expect("Can set read timeout for Redis connection");
    (pubsub_connection, secondary_redis_connection)
}

#[derive(Serialize)]
pub struct ErrorMessage {
    error: String,
}
impl ErrorMessage {
    fn new(msg: impl std::fmt::Display) -> Self {
        Self {
            error: msg.to_string(),
        }
    }
}

/// Recover from Errors by sending appropriate Warp::Rejections
pub fn handle_errors(
    rejection: warp::reject::Rejection,
) -> Result<impl warp::Reply, warp::reject::Rejection> {
    let err_txt = match rejection.cause() {
        Some(text) if text.to_string() == "Missing request header 'authorization'" => {
            "Error: Missing access token".to_string()
        }
        Some(text) => text.to_string(),
        None => "Error: Nonexistant endpoint".to_string(),
    };
    let json = warp::reply::json(&ErrorMessage::new(err_txt));
    Ok(warp::reply::with_status(
        json,
        warp::http::StatusCode::UNAUTHORIZED,
    ))
}

pub struct CustomError {}

impl CustomError {
    pub fn unauthorized_list() -> warp::reject::Rejection {
        warp::reject::custom("Error: Access to list not authorized")
    }
}
