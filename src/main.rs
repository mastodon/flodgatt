mod api;
mod common;
mod middleware;

use actix::prelude::*;
use actix_redis::RedisActor;
use actix_web::{http::header, middleware::cors::Cors, server, App};
use env_logger::Builder;
use log::info;
use std::net::SocketAddr;
use structopt::StructOpt;

const ENV_LOG_VARIABLE: &str = "STREAMING_API_LOG";

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long, default_value = "3666")]
    port: u16,
}

#[derive(Clone)]
pub struct AppState {
    redis: Addr<RedisActor>,
}

fn main() {
    Builder::from_env(ENV_LOG_VARIABLE).init();

    let args = Opt::from_args();

    info!("starting streaming api server");

    let addr: SocketAddr = ([127, 0, 0, 1], args.port).into();

    let sys = System::new("streaming-api-server");

    let redis_addr = RedisActor::start("127.0.0.1:6379");

    let app_state = AppState {
        redis: redis_addr.clone(),
    };

    server::new(move || vec![ws_endpoints(&app_state), http_endpoints(&app_state)])
        .bind(addr)
        .unwrap()
        .start();

    sys.run();
}

fn http_endpoints(app_state: &AppState) -> App<AppState> {
    use api::http;

    App::with_state(app_state.clone())
        .middleware(cors_middleware())
        .prefix("/api/v1")
        .resource("/streaming/user", |r| r.with(http::user::index))
        .resource("/streaming/public", |r| r.with(http::public::index))
        .resource("/streaming/public/local", |r| r.with(http::public::local))
        .resource("/streaming/direct", |r| r.with(http::direct::index))
        .resource("/streaming/hashtag", |r| r.with(http::hashtag::index))
        .resource("/streaming/hashtag/local", |r| r.with(http::hashtag::local))
        .resource("/streaming/list", |r| r.with(http::list::index))
}

fn ws_endpoints(app_state: &AppState) -> App<AppState> {
    use api::ws;

    App::with_state(app_state.clone()).resource("/api/v1/streaming", |r| r.with(ws::index))
}

fn cors_middleware() -> Cors {
    Cors::build()
        .allowed_origin("*")
        .allowed_methods(vec!["GET", "OPTIONS"])
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::CACHE_CONTROL])
        .finish()
}
