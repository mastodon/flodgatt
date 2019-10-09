use flodgatt::{
    config, dbg_and_die, err,
    parse_client_request::{sse, user, ws},
    redis_to_client_stream::{self, ClientAgent},
};
use log::warn;
use std::{collections::HashMap, env, net};
use warp::{ws::Ws2, Filter as WarpFilter};

fn main() {
    dotenv::from_filename(
        match env::var("ENV").ok().as_ref().map(String::as_str) {
        Some("production") => ".env.production",
        Some("development") | None => ".env",
            Some(_) => err::die_with_msg("Unknown ENV variable specified.\n    Valid options are: `production` or `development`."),
        }).ok();
    let env_vars_map: HashMap<_, _> = dotenv::vars().collect();
    let env_vars = config::EnvVar(env_vars_map);
    pretty_env_logger::init();
    let redis_cfg = config::RedisConfig::from_env(env_vars.clone());
    let cfg = config::DeploymentConfig::from_env(env_vars.clone());

    let postgres_cfg = config::PostgresConfig::from_env(env_vars.clone());

    let client_agent_sse = ClientAgent::blank(redis_cfg);
    let client_agent_ws = client_agent_sse.clone_with_shared_receiver();
    let pg_conn = user::PostgresConn::new(postgres_cfg);

    warn!("Streaming server initialized and ready to accept connections");

    // Server Sent Events
    let sse_update_interval = *cfg.ws_interval;
    let sse_routes = sse::extract_user_or_reject(pg_conn.clone())
        .and(warp::sse())
        .map(
            move |user: user::User, sse_connection_to_client: warp::sse::Sse| {
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
    let websocket_routes = ws::extract_user_or_reject(pg_conn.clone())
        .and(warp::ws::ws2())
        .map(move |user: user::User, ws: Ws2| {
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

    let server_addr = net::SocketAddr::new(*cfg.address, cfg.port.0);

    if let Some(_socket) = cfg.unix_socket.0.as_ref() {
        dbg_and_die!("Unix socket support not yet implemented");
    } else {
        warp::serve(websocket_routes.or(sse_routes).with(cors)).run(server_addr);
    }
}
