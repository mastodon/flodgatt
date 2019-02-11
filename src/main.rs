use actix_web::{server, App, HttpRequest, Responder};
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

    server::new(|| App::new().resource("/api/v1/streaming", |r| r.with(index)))
        .bind(SocketAddr::from(([127, 0, 0, 1], args.port)))
        .unwrap()
        .run();
}

fn index(_req: HttpRequest) -> impl Responder {
    "OMG! It works!"
}
