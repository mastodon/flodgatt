mod api;

use actix_web::{server, App};
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

fn main() {
    Builder::from_env(ENV_LOG_VARIABLE).init();

    let args = Opt::from_args();

    info!("starting streaming api server");

    let addr: SocketAddr = ([127, 0, 0, 1], args.port).into();

    use api::{http, ws};

    server::new(|| {
        App::new()
            .resource("/api/v1/streaming/user", |r| r.with(http::user::index))
            .resource("/api/v1/streaming/public", |r| r.with(http::public::index))
            .resource("/api/v1/streaming/public/local", |r| r.with(http::public::local))
            .resource("/api/v1/streaming/direct", |r| r.with(http::direct::index))
            .resource("/api/v1/streaming/hashtag", |r| r.with(http::hashtag::index))
            .resource("/api/v1/streaming/hashtag/local", |r| r.with(http::hashtag::local))
            .resource("/api/v1/streaming/list", |r| r.with(http::list::index))
            .resource("/api/v1/streaming", |r| r.with(ws::index))
    })
    .bind(addr)
    .unwrap()
    .shutdown_timeout(10)
    .run();
}
