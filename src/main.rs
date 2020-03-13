use flodgatt::{
    config, err,
    parse_client_request::{sse, user, ws},
    redis_to_client_stream::{self, ClientAgent},
};
use std::{collections::HashMap, env, fs, net, os::unix::fs::PermissionsExt};
use tokio::net::UnixListener;
use warp::{path, ws::Ws2, Filter};

fn main() {
    dotenv::from_filename(
        match env::var("ENV").ok().as_ref().map(String::as_str) {
        Some("production") => ".env.production",
        Some("development") | None => ".env",
            Some(_) => err::die_with_msg("Unknown ENV variable specified.\n    Valid options are: `production` or `development`."),
        }).ok();
    let env_vars_map: HashMap<_, _> = dotenv::vars().collect();
    let env_vars = config::EnvVar::new(env_vars_map);
    pretty_env_logger::init();

    log::warn!(
        "Flodgatt recognized the following environmental variables:{}",
        env_vars.clone()
    );
    let redis_cfg = config::RedisConfig::from_env(env_vars.clone());
    let cfg = config::DeploymentConfig::from_env(env_vars.clone());

    let postgres_cfg = config::PostgresConfig::from_env(env_vars.clone());

    let client_agent_sse = ClientAgent::blank(redis_cfg);
    let client_agent_ws = client_agent_sse.clone_with_shared_receiver();
    let pg_pool = user::PgPool::new(postgres_cfg);

    log::warn!("Streaming server initialized and ready to accept connections");

    // Server Sent Events
    let sse_update_interval = *cfg.ws_interval;
    let sse_routes = sse::extract_user_or_reject(pg_pool.clone())
        .and(warp::sse())
        .map(
            move |user: user::User, sse_connection_to_client: warp::sse::Sse| {
                log::info!("Incoming SSE request");
                // Create a new ClientAgent
                let mut client_agent = client_agent_sse.clone_with_shared_receiver();
                // Assign ClientAgent to generate stream of updates for the user/timeline pair
                client_agent.init_for_user(user);
                // send the updates through the SSE connection
                redis_to_client_stream::send_updates_to_sse(
                    client_agent,
                    sse_connection_to_client,
                    sse_update_interval,
                )
            },
        )
        .with(warp::reply::with::header("Connection", "keep-alive"))
        .recover(err::handle_errors);

    // WebSocket
    let ws_update_interval = *cfg.ws_interval;
    let websocket_routes = ws::extract_user_or_reject(pg_pool.clone())
        .and(warp::ws::ws2())
        .map(move |user: user::User, ws: Ws2| {
            log::info!("Incoming websocket request");
            let token = user.access_token.clone();
            // Create a new ClientAgent
            let mut client_agent = client_agent_ws.clone_with_shared_receiver();
            // Assign that agent to generate a stream of updates for the user/timeline pair
            client_agent.init_for_user(user);
            // send the updates through the WS connection (along with the User's access_token
            // which is sent for security)

            (
                ws.on_upgrade(move |socket| {
                    redis_to_client_stream::send_updates_to_ws(
                        socket,
                        client_agent,
                        ws_update_interval,
                    )
                }),
                token,
            )
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(cfg.cors.allowed_methods)
        .allow_headers(cfg.cors.allowed_headers);

    let health = warp::path!("api" / "v1" / "streaming" / "health").map(|| "OK");

    if let Some(socket) = &*cfg.unix_socket {
        log::warn!("Using Unix socket {}", socket);
        fs::remove_file(socket).unwrap_or_default();
        let incoming = UnixListener::bind(socket).unwrap().incoming();

        fs::set_permissions(socket, PermissionsExt::from_mode(0o666)).unwrap();

        warp::serve(health.or(websocket_routes.or(sse_routes).with(cors))).run_incoming(incoming);
    } else {
        let server_addr = net::SocketAddr::new(*cfg.address, cfg.port.0);
        warp::serve(health.or(websocket_routes.or(sse_routes).with(cors))).run(server_addr);
    }
}
