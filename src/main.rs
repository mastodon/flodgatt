use flodgatt::config;
use flodgatt::request::{Handler, Subscription};
use flodgatt::response::{RedisManager, SseStream, WsStream};
use flodgatt::Error;

use futures::{future::lazy, stream::Stream as _};
use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::time::Instant;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tokio::timer::Interval;
use warp::ws::Ws2;
use warp::Filter;

fn main() -> Result<(), Error> {
    config::merge_dotenv()?;
    pretty_env_logger::try_init_timed()?;
    let (postgres_cfg, redis_cfg, cfg) = config::from_env(dotenv::vars().collect())?;
    let poll_freq = *redis_cfg.polling_interval;

    let request = Handler::new(&postgres_cfg, *cfg.whitelist_mode)?;
    let shared_manager = RedisManager::try_from(&redis_cfg)?.into_arc();

    // Server Sent Events
    let sse_manager = shared_manager.clone();
    let sse = request
        .sse_subscription()
        .and(warp::sse())
        .map(move |subscription: Subscription, sse: warp::sse::Sse| {
            log::info!("Incoming SSE request for {:?}", subscription.timeline);
            let mut manager = sse_manager.lock().unwrap_or_else(RedisManager::recover);
            let (event_tx, event_rx) = mpsc::channel(10);
            manager.subscribe(&subscription, event_tx);
            let sse_stream = SseStream::new(subscription);
            sse_stream.send_events(sse, event_rx)
        })
        .with(warp::reply::with::header("Connection", "keep-alive"));

    // WebSocket
    let ws_manager = shared_manager.clone();
    let ws = request
        .ws_subscription()
        .and(warp::ws::ws2())
        .map(move |subscription: Subscription, ws: Ws2| {
            log::info!("Incoming websocket request for {:?}", subscription.timeline);
            let mut manager = ws_manager.lock().unwrap_or_else(RedisManager::recover);
            let (event_tx, event_rx) = mpsc::channel(10);
            manager.subscribe(&subscription, event_tx);
            let token = subscription.access_token.clone().unwrap_or_default(); // token sent for security
            let ws_stream = WsStream::new(subscription);

            (
                ws.on_upgrade(move |ws| ws_stream.send_to(ws, event_rx)),
                token,
            )
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    #[cfg(feature = "stub_status")]
    #[rustfmt::skip]
    let status = {
        let (r1, r3) = (shared_manager.clone(), shared_manager.clone());
        request.health().map(|| "OK")
            .or(request.status()
                .map(move || r1.lock().unwrap_or_else(RedisManager::recover).count()))
            .or(request.status_per_timeline()
                .map(move || r3.lock().unwrap_or_else(RedisManager::recover).list()))
    };
    #[cfg(not(feature = "stub_status"))]
    let status = request.health().map(|| "OK");

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(cfg.cors.allowed_methods)
        .allow_headers(cfg.cors.allowed_headers);

    let streaming_server = move || {
        let manager = shared_manager.clone();
        let stream = Interval::new(Instant::now(), poll_freq)
            .map_err(|e| log::error!("{}", e))
            .for_each(move |_| {
                let mut manager = manager.lock().unwrap_or_else(RedisManager::recover);
                manager.poll_broadcast().map_err(Error::log)
            });

        warp::spawn(lazy(move || stream));
        warp::serve(ws.or(sse).with(cors).or(status).recover(Handler::err))
    };

    if let Some(socket) = &*cfg.unix_socket {
        log::info!("Using Unix socket {}", socket);
        fs::remove_file(socket).unwrap_or_default();
        let incoming = UnixListener::bind(socket)?.incoming();
        fs::set_permissions(socket, PermissionsExt::from_mode(0o666))?;
        tokio::run(lazy(|| streaming_server().serve_incoming(incoming)));
    } else {
        let server_addr = SocketAddr::new(*cfg.address, *cfg.port);
        tokio::run(lazy(move || streaming_server().bind(server_addr)));
    }
    Err(Error::Unrecoverable) // only reached if poll_broadcast encounters an unrecoverable error
}
