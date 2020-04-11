use flodgatt::{
    config::{DeploymentConfig, EnvVar, PostgresConfig, RedisConfig},
    err::FatalErr,
    messages::Event,
    parse_client_request::{PgPool, Subscription, Timeline},
    redis_to_client_stream::{Receiver, SseStream, WsStream},
};
use std::{env, fs, net::SocketAddr, os::unix::fs::PermissionsExt};
use tokio::{
    net::UnixListener,
    sync::{mpsc, watch},
};
use warp::{http::StatusCode, path, ws::Ws2, Filter, Rejection};

fn main() -> Result<(), FatalErr> {
    dotenv::from_filename(match env::var("ENV").ok().as_deref() {
        Some("production") => ".env.production",
        Some("development") | None => ".env",
        Some(unsupported) => EnvVar::err("ENV", unsupported, "`production` or `development`"),
    })
    .ok();
    let env_vars = EnvVar::new(dotenv::vars().collect());
    pretty_env_logger::init();
    log::info!("Environmental variables Flodgatt received: {}", &env_vars);

    let postgres_cfg = PostgresConfig::from_env(env_vars.clone());
    let redis_cfg = RedisConfig::from_env(env_vars.clone());
    let cfg = DeploymentConfig::from_env(env_vars);

    let pg_pool = PgPool::new(postgres_cfg);
    let (event_tx, event_rx) = watch::channel((Timeline::empty(), Event::Ping));
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let poll_freq = *redis_cfg.polling_interval;
    let receiver = Receiver::try_from(redis_cfg, event_tx, cmd_rx)?.into_arc();
    log::info!("Streaming server initialized and ready to accept connections");

    // Server Sent Events
    let sse_receiver = receiver.clone();
    let (sse_rx, sse_cmd_tx) = (event_rx.clone(), cmd_tx.clone());
    let whitelist_mode = *cfg.whitelist_mode;
    let sse_routes = Subscription::from_sse_query(pg_pool.clone(), whitelist_mode)
        .and(warp::sse())
        .map(
            move |subscription: Subscription, sse_connection_to_client: warp::sse::Sse| {
                log::info!("Incoming SSE request for {:?}", subscription.timeline);
                {
                    let mut receiver = sse_receiver.lock().unwrap_or_else(Receiver::recover);
                    receiver.subscribe(&subscription).unwrap_or_else(|e| {
                        log::error!("Could not subscribe to the Redis channel: {}", e)
                    });
                }
                let cmd_tx = sse_cmd_tx.clone();
                let sse_rx = sse_rx.clone();
                // send the updates through the SSE connection
                SseStream::send_events(sse_connection_to_client, cmd_tx, subscription, sse_rx)
            },
        )
        .with(warp::reply::with::header("Connection", "keep-alive"));

    // WebSocket
    let ws_receiver = receiver.clone();
    let whitelist_mode = *cfg.whitelist_mode;
    let ws_routes = Subscription::from_ws_request(pg_pool, whitelist_mode)
        .and(warp::ws::ws2())
        .map(move |subscription: Subscription, ws: Ws2| {
            log::info!("Incoming websocket request for {:?}", subscription.timeline);
            {
                let mut receiver = ws_receiver.lock().unwrap_or_else(Receiver::recover);

                receiver.subscribe(&subscription).unwrap_or_else(|e| {
                    log::error!("Could not subscribe to the Redis channel: {}", e)
                });
            }
            let cmd_tx = cmd_tx.clone();
            let ws_rx = event_rx.clone();
            let token = subscription
                .clone()
                .access_token
                .unwrap_or_else(String::new);

            // send the updates through the WS connection (along with the access_token, for security)
            (
                ws.on_upgrade(move |ws| WsStream::new(ws, cmd_tx, subscription).send_events(ws_rx)),
                token,
            )
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(cfg.cors.allowed_methods)
        .allow_headers(cfg.cors.allowed_headers);

    #[cfg(feature = "stub_status")]
    let status_endpoints = {
        let (r1, r3) = (receiver.clone(), receiver.clone());
        warp::path!("api" / "v1" / "streaming" / "health")
            .map(|| "OK")
            .or(warp::path!("api" / "v1" / "streaming" / "status")
                .and(warp::path::end())
                .map(move || r1.lock().unwrap_or_else(Receiver::recover).count()))
            .or(
                warp::path!("api" / "v1" / "streaming" / "status" / "per_timeline")
                    .map(move || r3.lock().unwrap_or_else(Receiver::recover).list()),
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
            let receiver = receiver.clone();

            warp::spawn(lazy(move || {
                tokio::timer::Interval::new(Instant::now(), poll_freq)
                    .map_err(|e| log::error!("{}", e))
                    .for_each(move |_| {
                        let mut receiver = receiver.lock().unwrap_or_else(Receiver::recover);
                        receiver.poll_broadcast().unwrap_or_else(FatalErr::exit);
                        Ok(())
                    })
            }));

            warp::serve(ws_routes.or(sse_routes).with(cors).or(status_endpoints)).bind(server_addr)
        }));
    };
    Ok(())
}
