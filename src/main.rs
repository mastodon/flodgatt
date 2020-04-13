use flodgatt::config;
use flodgatt::err::FatalErr;
use flodgatt::messages::Event;
use flodgatt::request::{self, Subscription, Timeline};
use flodgatt::response::redis;
use flodgatt::response::stream;

use futures::{future::lazy, stream::Stream as _Stream};
use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::time::Instant;
use tokio::net::UnixListener;
use tokio::sync::{mpsc, watch};
use tokio::timer::Interval;
use warp::http::StatusCode;
use warp::path;
use warp::ws::Ws2;
use warp::{Filter, Rejection};

fn main() -> Result<(), FatalErr> {
    config::merge_dotenv()?;
    pretty_env_logger::try_init()?;
    let (postgres_cfg, redis_cfg, cfg) = config::from_env(dotenv::vars().collect());

    // Create channels to communicate between threads
    let (event_tx, event_rx) = watch::channel((Timeline::empty(), Event::Ping));
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

    let request_handler = request::Handler::new(postgres_cfg, *cfg.whitelist_mode);
    let poll_freq = *redis_cfg.polling_interval;
    let shared_manager = redis::Manager::try_from(redis_cfg, event_tx, cmd_rx)?.into_arc();

    // Server Sent Events
    let sse_manager = shared_manager.clone();
    let (sse_rx, sse_cmd_tx) = (event_rx.clone(), cmd_tx.clone());

    let sse = request_handler
        .parse_sse_request()
        .and(warp::sse())
        .map(
            move |subscription: Subscription, client_conn: warp::sse::Sse| {
                log::info!("Incoming SSE request for {:?}", subscription.timeline);
                {
                    let mut manager = sse_manager.lock().unwrap_or_else(redis::Manager::recover);
                    manager.subscribe(&subscription);
                }

                stream::Sse::send_events(
                    client_conn,
                    sse_cmd_tx.clone(),
                    subscription,
                    sse_rx.clone(),
                )
            },
        )
        .with(warp::reply::with::header("Connection", "keep-alive"));

    // WebSocket
    let ws_manager = shared_manager.clone();
    let ws = request_handler
        .parse_ws_request()
        .and(warp::ws::ws2())
        .map(move |subscription: Subscription, ws: Ws2| {
            log::info!("Incoming websocket request for {:?}", subscription.timeline);
            {
                let mut manager = ws_manager.lock().unwrap_or_else(redis::Manager::recover);
                manager.subscribe(&subscription);
            }
            let token = subscription.access_token.clone().unwrap_or_default(); // token sent for security
            let ws_stream = stream::Ws::new(cmd_tx.clone(), event_rx.clone(), subscription);

            (ws.on_upgrade(move |ws| ws_stream.send_to(ws)), token)
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(cfg.cors.allowed_methods)
        .allow_headers(cfg.cors.allowed_headers);

    // TODO -- extract to separate file
    #[cfg(feature = "stub_status")]
    let status = {
        let (r1, r3) = (shared_manager.clone(), shared_manager.clone());
        warp::path!("api" / "v1" / "streaming" / "health")
            .map(|| "OK")
            .or(warp::path!("api" / "v1" / "streaming" / "status")
                .and(warp::path::end())
                .map(move || r1.lock().unwrap_or_else(redis::Manager::recover).count()))
            .or(
                warp::path!("api" / "v1" / "streaming" / "status" / "per_timeline")
                    .map(move || r3.lock().unwrap_or_else(redis::Manager::recover).list()),
            )
    };
    #[cfg(not(feature = "stub_status"))]
    let status = warp::path!("api" / "v1" / "streaming" / "health").map(|| "OK");

    let streaming_server = move || {
        let manager = shared_manager.clone();
        let stream = Interval::new(Instant::now(), poll_freq)
            .map_err(|e| log::error!("{}", e))
            .for_each(move |_| {
                let mut manager = manager.lock().unwrap_or_else(redis::Manager::recover);
                manager.poll_broadcast().unwrap_or_else(FatalErr::exit);
                Ok(())
            });
        warp::spawn(lazy(move || stream));
        warp::serve(ws.or(sse).with(cors).or(status).recover(recover))
    };

    if let Some(socket) = &*cfg.unix_socket {
        log::info!("Using Unix socket {}", socket);
        fs::remove_file(socket).unwrap_or_default();
        let incoming = UnixListener::bind(socket).expect("TODO").incoming();
        fs::set_permissions(socket, PermissionsExt::from_mode(0o666)).expect("TODO");

        tokio::run(lazy(|| streaming_server().serve_incoming(incoming)));
    } else {
        let server_addr = SocketAddr::new(*cfg.address, *cfg.port);
        tokio::run(lazy(move || streaming_server().bind(server_addr)));
    }
    Ok(())
}

// TODO -- extract to separate file
fn recover(r: Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    let json_err = match r.cause() {
        Some(text) if text.to_string() == "Missing request header 'authorization'" => {
            warp::reply::json(&"Error: Missing access token".to_string())
        }
        Some(text) => warp::reply::json(&text.to_string()),
        None => warp::reply::json(&"Error: Nonexistant endpoint".to_string()),
    };
    Ok(warp::reply::with_status(json_err, StatusCode::UNAUTHORIZED))
}
