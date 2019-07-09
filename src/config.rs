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
const DEFAULT_POSTGRES_ADDR: &str = "postgres://@localhost/mastodon_development";
const DEFAULT_REDIS_ADDR: &str = "127.0.0.1:6379";
const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:4000";

const DEFAULT_SSE_UPDATE_INTERVAL: u64 = 100;
const DEFAULT_WS_UPDATE_INTERVAL: u64 = 100;
const DEFAULT_REDIS_POLL_INTERVAL: u64 = 100;

lazy_static! {
    static ref POSTGRES_ADDR: String = env::var("POSTGRESS_ADDR").unwrap_or_else(|_| {
        let mut postgres_addr = DEFAULT_POSTGRES_ADDR.to_string();
        postgres_addr.insert_str(11,
             &env::var("USER").unwrap_or_else(|_| {
                 warn!("No USER env variable set.  Connecting to Postgress with default `postgres` user");
                 "postgres".to_string()
             }).as_str()
        );
        postgres_addr
    });

    static ref REDIS_ADDR: String = env::var("REDIS_ADDR").unwrap_or_else(|_| DEFAULT_REDIS_ADDR.to_owned());

    pub static ref SERVER_ADDR: net::SocketAddr = env::var("SERVER_ADDR")
        .unwrap_or_else(|_| DEFAULT_SERVER_ADDR.to_owned())
        .parse()
        .expect("static string");

    /// Interval, in ms, at which the `ClientAgent` polls the `Receiver` for updates to send via SSE.
    pub static ref SSE_UPDATE_INTERVAL: u64 = env::var("SSE_UPDATE_INTERVAL")
        .map(|s| s.parse().expect("Valid config"))
        .unwrap_or(DEFAULT_SSE_UPDATE_INTERVAL);
    /// Interval, in ms, at which the `ClientAgent` polls the `Receiver` for updates to send via WS.
    pub static ref WS_UPDATE_INTERVAL: u64 = env::var("WS_UPDATE_INTERVAL")
        .map(|s| s.parse().expect("Valid config"))
        .unwrap_or(DEFAULT_WS_UPDATE_INTERVAL);
    /// Interval, in ms, at which the `Receiver` polls Redis.
    /// **NOTE**:  Polling Redis is much more time consuming than polling the `Receiver`
    ///            (on the order of 10ms rather than 50μs).  Thus, changing this setting
    ///            would be a good place to start for performance improvements at the cost
    ///            of delaying all updates.
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
    pretty_env_logger::init();
    dotenv().ok();
}

/// Configure Postgres and return a connection
pub fn postgres() -> postgres::Connection {
    postgres::Connection::connect(POSTGRES_ADDR.to_string(), postgres::TlsMode::None)
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
