//! Configuration settings for servers and databases
use dotenv::dotenv;
use log::warn;
use std::{env, net, time};

/// Configure CORS for the API server
pub fn cross_origin_resource_sharing() -> warp::filters::cors::Cors {
    warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "OPTIONS"])
        .allow_headers(vec!["Authorization", "Accept", "Cache-Control"])
}

/// Initialize logging and read values from `src/.env`
pub fn logging_and_env() {
    pretty_env_logger::init();
    dotenv().ok();
}

/// Configure Postgres and return a connection
pub fn postgres() -> postgres::Connection {
    let postgres_addr = env::var("POSTGRESS_ADDR").unwrap_or_else(|_| {
        format!(
            "postgres://{}@localhost/mastodon_development",
            env::var("USER").unwrap_or_else(|_| {
                warn!("No USER env variable set.  Connecting to Postgress with default `postgres` user");
                "postgres".to_owned()
            })
        )
    });
    postgres::Connection::connect(postgres_addr, postgres::TlsMode::None)
        .expect("Can connect to local Postgres")
}

pub fn redis_addr() -> (net::TcpStream, net::TcpStream) {
    let redis_addr = env::var("REDIS_ADDR").unwrap_or_else(|_| "127.0.0.1:6379".to_string());
    let pubsub_connection = net::TcpStream::connect(&redis_addr).expect("Can connect to Redis");
    pubsub_connection
        .set_read_timeout(Some(time::Duration::from_millis(10)))
        .expect("Can set read timeout for Redis connection");
    let secondary_redis_connection =
        net::TcpStream::connect(&redis_addr).expect("Can connect to Redis");
    secondary_redis_connection
        .set_read_timeout(Some(time::Duration::from_millis(10)))
        .expect("Can set read timeout for Redis connection");
    (pubsub_connection, secondary_redis_connection)
}

pub fn socket_address() -> net::SocketAddr {
    env::var("SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_owned())
        .parse()
        .expect("static string")
}
