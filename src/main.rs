use flodgatt::config;
use flodgatt::err::FatalErr;
use flodgatt::messages::Event;
use flodgatt::request::{PgPool, Subscription, Timeline};
use flodgatt::response::redis;
use flodgatt::response::stream;

use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use tokio::net::UnixListener;
use tokio::sync::{mpsc, watch};
use warp::http::StatusCode;
use warp::path;
use warp::ws::Ws2;
use warp::{Filter, Rejection};

fn main() -> Result<(), FatalErr> {
    config::merge_dotenv()?;
    pretty_env_logger::try_init()?;

    let (postgres_cfg, redis_cfg, cfg) = config::from_env(dotenv::vars().collect());
    let (event_tx, event_rx) = watch::channel((Timeline::empty(), Event::Ping));
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

    let shared_pg_conn = PgPool::new(postgres_cfg, *cfg.whitelist_mode);
    let poll_freq = *redis_cfg.polling_interval;
    let manager = redis::Manager::try_from(redis_cfg, event_tx, cmd_rx)?.into_arc();

    // Server Sent Events
    let sse_manager = manager.clone();
    let (sse_rx, sse_cmd_tx) = (event_rx.clone(), cmd_tx.clone());
    let sse_routes = Subscription::from_sse_request(shared_pg_conn.clone())
        .and(warp::sse())
        .map(
            move |subscription: Subscription, client_conn: warp::sse::Sse| {
                log::info!("Incoming SSE request for {:?}", subscription.timeline);
                {
                    let mut manager = sse_manager.lock().unwrap_or_else(redis::Manager::recover);
                    manager.subscribe(&subscription).unwrap_or_else(|e| {
                        log::error!("Could not subscribe to the Redis channel: {}", e)
                    });
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
    let ws_manager = manager.clone();
    let ws_routes = Subscription::from_ws_request(shared_pg_conn)
        .and(warp::ws::ws2())
        .map(move |subscription: Subscription, ws: Ws2| {
            log::info!("Incoming websocket request for {:?}", subscription.timeline);
            {
                let mut manager = ws_manager.lock().unwrap_or_else(redis::Manager::recover);

                manager.subscribe(&subscription).unwrap_or_else(|e| {
                    log::error!("Could not subscribe to the Redis channel: {}", e)
                });
            }
            let cmd_tx = cmd_tx.clone();
            let ws_rx = event_rx.clone();
            let token = subscription
                .clone()
                .access_token
                .unwrap_or_else(String::new);

            let ws_response_stream = ws
                .on_upgrade(move |ws| stream::Ws::new(ws, cmd_tx, subscription).send_events(ws_rx));

            (ws_response_stream, token)
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(cfg.cors.allowed_methods)
        .allow_headers(cfg.cors.allowed_headers);

    #[cfg(feature = "stub_status")]
    let status_endpoints = {
        let (r1, r3) = (manager.clone(), manager.clone());
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
    let status_endpoints = warp::path!("api" / "v1" / "streaming" / "health").map(|| "OK");

    if let Some(socket) = &*cfg.unix_socket {
        log::info!("Using Unix socket {}", socket);
        fs::remove_file(socket).unwrap_or_default();
        let incoming = UnixListener::bind(socket).unwrap().incoming();
        fs::set_permissions(socket, PermissionsExt::from_mode(0o666)).unwrap();

        warp::serve(
            ws_routes
                .or(sse_routes)
                .with(cors)
                .or(status_endpoints)
                .recover(|r: Rejection| {
                    let json_err = match r.cause() {
                        Some(text)
                            if text.to_string() == "Missing request header 'authorization'" =>
                        {
                            warp::reply::json(&"Error: Missing access token".to_string())
                        }
                        Some(text) => warp::reply::json(&text.to_string()),
                        None => warp::reply::json(&"Error: Nonexistant endpoint".to_string()),
                    };
                    Ok(warp::reply::with_status(json_err, StatusCode::UNAUTHORIZED))
                }),
        )
        .run_incoming(incoming);
    } else {
        use futures::{future::lazy, stream::Stream as _Stream};
        use std::time::Instant;

        let server_addr = SocketAddr::new(*cfg.address, *cfg.port);

        tokio::run(lazy(move || {
            let receiver = manager.clone();

            warp::spawn(lazy(move || {
                tokio::timer::Interval::new(Instant::now(), poll_freq)
                    .map_err(|e| log::error!("{}", e))
                    .for_each(move |_| {
                        let mut receiver = receiver.lock().unwrap_or_else(redis::Manager::recover);
                        receiver.poll_broadcast().unwrap_or_else(FatalErr::exit);
                        Ok(())
                    })
            }));

            warp::serve(ws_routes.or(sse_routes).with(cors).or(status_endpoints)).bind(server_addr)
        }));
    };
    Ok(())
}
