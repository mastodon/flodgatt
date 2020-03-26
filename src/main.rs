use flodgatt::{
    config::{DeploymentConfig, EnvVar, PostgresConfig, RedisConfig},
    parse_client_request::{PgPool, Subscription},
    redis_to_client_stream::{ClientAgent, EventStream},
};
use std::{collections::HashMap, env, fs, net, os::unix::fs::PermissionsExt};
use tokio::net::UnixListener;
use warp::{path, ws::Ws2, Filter};

fn main() {
    dotenv::from_filename(match env::var("ENV").ok().as_ref().map(String::as_str) {
        Some("production") => ".env.production",
        Some("development") | None => ".env",
        Some(unsupported) => EnvVar::err("ENV", unsupported, "`production` or `development`"),
    })
    .ok();
    let env_vars_map: HashMap<_, _> = dotenv::vars().collect();
    let env_vars = EnvVar::new(env_vars_map);
    pretty_env_logger::init();

    log::info!(
        "Flodgatt recognized the following environmental variables:{}",
        env_vars.clone()
    );
    let redis_cfg = RedisConfig::from_env(env_vars.clone());
    let cfg = DeploymentConfig::from_env(env_vars.clone());

    let postgres_cfg = PostgresConfig::from_env(env_vars.clone());
    let pg_pool = PgPool::new(postgres_cfg);

    let client_agent_sse = ClientAgent::blank(redis_cfg);
    let client_agent_ws = client_agent_sse.clone_with_shared_receiver();

    log::info!("Streaming server initialized and ready to accept connections");

    // Server Sent Events
    let (sse_interval, whitelist_mode) = (*cfg.sse_interval, *cfg.whitelist_mode);
    let sse_routes = Subscription::from_sse_query(pg_pool.clone(), whitelist_mode)
        .and(warp::sse())
        .map(
            move |subscription: Subscription, sse_connection_to_client: warp::sse::Sse| {
                log::info!("Incoming SSE request for {:?}", subscription.timeline);
                // Create a new ClientAgent
                let mut client_agent = client_agent_sse.clone_with_shared_receiver();
                // Assign ClientAgent to generate stream of updates for the user/timeline pair
                client_agent.init_for_user(subscription);
                // send the updates through the SSE connection
                EventStream::to_sse(client_agent, sse_connection_to_client, sse_interval)
            },
        )
        .with(warp::reply::with::header("Connection", "keep-alive"));

    // WebSocket
    let (ws_update_interval, whitelist_mode) = (*cfg.ws_interval, *cfg.whitelist_mode);
    let websocket_routes = Subscription::from_ws_request(pg_pool.clone(), whitelist_mode)
        .and(warp::ws::ws2())
        .map(move |subscription: Subscription, ws: Ws2| {
            log::info!("Incoming websocket request for {:?}", subscription.timeline);

            let token = subscription.access_token.clone();
            // Create a new ClientAgent
            let mut client_agent = client_agent_ws.clone_with_shared_receiver();
            // Assign that agent to generate a stream of updates for the user/timeline pair
            client_agent.init_for_user(subscription);
            // send the updates through the WS connection (along with the User's access_token
            // which is sent for security)
            (
                ws.on_upgrade(move |socket| {
                    EventStream::to_ws(socket, client_agent, ws_update_interval)
                }),
                token.unwrap_or_else(String::new),
            )
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(cfg.cors.allowed_methods)
        .allow_headers(cfg.cors.allowed_headers);

    let health = warp::path!("api" / "v1" / "streaming" / "health").map(|| "OK");

    if let Some(socket) = &*cfg.unix_socket {
        log::info!("Using Unix socket {}", socket);
        fs::remove_file(socket).unwrap_or_default();
        let incoming = UnixListener::bind(socket).unwrap().incoming();

        fs::set_permissions(socket, PermissionsExt::from_mode(0o666)).unwrap();

        warp::serve(
            health.or(websocket_routes.or(sse_routes).with(cors).recover(
                |rejection: warp::reject::Rejection| {
                    let err_txt = match rejection.cause() {
                        Some(text)
                            if text.to_string() == "Missing request header 'authorization'" =>
                        {
                            "Error: Missing access token".to_string()
                        }
                        Some(text) => text.to_string(),
                        None => "Error: Nonexistant endpoint".to_string(),
                    };
                    let json = warp::reply::json(&err_txt);

                    Ok(warp::reply::with_status(
                        json,
                        warp::http::StatusCode::UNAUTHORIZED,
                    ))
                },
            )),
        )
        .run_incoming(incoming);
    } else {
        let server_addr = net::SocketAddr::new(*cfg.address, cfg.port.0);
        warp::serve(health.or(websocket_routes.or(sse_routes).with(cors))).run(server_addr);
    }
}
