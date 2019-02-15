mod api;
mod common;
mod middleware;

use actix::prelude::*;
use actix_redis::RedisActor;
use actix_web::{
    http::{header, Method},
    middleware::cors::Cors,
    server, App, HttpResponse,
};
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

    server::new(move || endpoints(&app_state)).bind(addr).unwrap().start();

    sys.run();
}

fn endpoints(app_state: &AppState) -> App<AppState> {
    use api::http;
    use api::ws;

    App::with_state(app_state.clone())
        .prefix("/api/v1")
        .resource("/streaming", |r| r.with(ws::index))
        .resource("/streaming/health", |r| {
            r.middleware(cors_middleware());
            r.get().f(|_| HttpResponse::Ok())
        })
        .resource("/streaming/user", |r| {
            r.middleware(cors_middleware());
            r.get().with(http::user::index)
        })
        .resource("/streaming/public", |r| {
            r.middleware(cors_middleware());
            r.get().with(http::public::index)
        })
        .resource("/streaming/public/local", |r| {
            r.middleware(cors_middleware());
            r.get().with(http::public::local)
        })
        .resource("/streaming/direct", |r| {
            r.middleware(cors_middleware());
            r.get().with(http::direct::index)
        })
        .resource("/streaming/hashtag", |r| {
            r.middleware(cors_middleware());
            r.get().with(http::hashtag::index)
        })
        .resource("/streaming/hashtag/local", |r| {
            r.middleware(cors_middleware());
            r.get().with(http::hashtag::local)
        })
        .resource("/streaming/list", |r| {
            r.middleware(cors_middleware());
            r.get().with(http::list::index)
        })
}

fn cors_middleware() -> Cors {
    Cors::build()
        .allowed_origin("*")
        .allowed_methods(vec!["GET", "OPTIONS"])
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::CACHE_CONTROL])
        .finish()
}
