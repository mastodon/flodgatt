use flodgatt::{
    config::{DeploymentConfig, EnvVar, PostgresConfig, RedisConfig},
    parse_client_request::{PgPool, Subscription},
    redis_to_client_stream::{ClientAgent, EventStream, Receiver},
};
use std::{env, fs, net::SocketAddr, os::unix::fs::PermissionsExt};
use tokio::net::UnixListener;
use warp::{http::StatusCode, path, ws::Ws2, Filter, Rejection};

fn main() {
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

    let sharable_receiver = Receiver::try_from(redis_cfg)
        .unwrap_or_else(|e| {
            log::error!("{}\nFlodgatt shutting down...", e);
            std::process::exit(1);
        })
        .into_arc();
    log::info!("Streaming server initialized and ready to accept connections");

    // Server Sent Events
    let sse_receiver = sharable_receiver.clone();
    let (sse_interval, whitelist_mode) = (*cfg.sse_interval, *cfg.whitelist_mode);
    let sse_routes = Subscription::from_sse_query(pg_pool.clone(), whitelist_mode)
        .and(warp::sse())
        .map(
            move |subscription: Subscription, sse_connection_to_client: warp::sse::Sse| {
                log::info!("Incoming SSE request for {:?}", subscription.timeline);
                let mut client_agent = ClientAgent::new(sse_receiver.clone(), &subscription);
                client_agent.subscribe();

                // send the updates through the SSE connection
                EventStream::send_to_sse(client_agent, sse_connection_to_client, sse_interval)
            },
        )
        .with(warp::reply::with::header("Connection", "keep-alive"));

    // WebSocket
    let ws_receiver = sharable_receiver.clone();
    let (ws_update_interval, whitelist_mode) = (*cfg.ws_interval, *cfg.whitelist_mode);
    let ws_routes = Subscription::from_ws_request(pg_pool, whitelist_mode)
        .and(warp::ws::ws2())
        .map(move |subscription: Subscription, ws: Ws2| {
            log::info!("Incoming websocket request for {:?}", subscription.timeline);
            let mut client_agent = ClientAgent::new(ws_receiver.clone(), &subscription);
            client_agent.subscribe();

            // send the updates through the WS connection
            // (along with the User's access_token which is sent for security)
            (
                ws.on_upgrade(move |s| {
                    EventStream::send_to_ws(s, client_agent, ws_update_interval)
                }),
                subscription.access_token.unwrap_or_else(String::new),
            )
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(cfg.cors.allowed_methods)
        .allow_headers(cfg.cors.allowed_headers);

    let health = warp::path!("api" / "v1" / "streaming" / "health").map(|| "OK");
    let stats_receiver = sharable_receiver.clone();
    let status = warp::path!("api" / "v1" / "streaming" / "status")
        .and(warp::path::end())
        .map(move || stats_receiver.lock().expect("TODO").count_connections());
    let stats_receiver = sharable_receiver.clone();
    let status_queue_len = warp::path!("api" / "v1" / "streaming" / "status" / "queue")
        .map(move || stats_receiver.lock().expect("TODO").queue_length());
    let stats_receiver = sharable_receiver.clone();
    let status_per_timeline = warp::path!("api" / "v1" / "streaming" / "status" / "per_timeline")
        .map(move || stats_receiver.lock().expect("TODO").list_connections());

    if let Some(socket) = &*cfg.unix_socket {
        log::info!("Using Unix socket {}", socket);
        fs::remove_file(socket).unwrap_or_default();
        let incoming = UnixListener::bind(socket).unwrap().incoming();
        fs::set_permissions(socket, PermissionsExt::from_mode(0o666)).unwrap();

        warp::serve(
            health.or(
                status.or(status_per_timeline.or(status_queue_len.or(ws_routes
                    .or(sse_routes)
                    .with(cors)
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
                    })))),
            ),
        )
        .run_incoming(incoming);
    } else {
        let server_addr = SocketAddr::new(*cfg.address, *cfg.port);
        warp::serve(health.or(
            status.or(
                status_per_timeline.or(status_queue_len.or(ws_routes.or(sse_routes).with(cors))),
            ),
        ))
        .run(server_addr);
    };
}
