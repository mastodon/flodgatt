use flodgatt::{
    config,
    parse_client_request::{sse, user, ws},
    redis_to_client_stream,
    redis_to_client_stream::ClientAgent,
};
use log::warn;
use warp::{ws::Ws2, Filter as WarpFilter};

fn main() {
    config::logging_and_env();
    let client_agent_sse = ClientAgent::blank();
    let client_agent_ws = client_agent_sse.clone_with_shared_receiver();

    warn!("Streaming server initialized and ready to accept connections");

    // Server Sent Events
    let sse_routes = sse::extract_user_or_reject()
        .and(warp::sse())
        .map(
            move |user: user::User, sse_connection_to_client: warp::sse::Sse| {
                // Create a new ClientAgent
                let mut client_agent = client_agent_sse.clone_with_shared_receiver();
                // Assign ClientAgent to generate stream of updates for the user/timeline pair
                client_agent.init_for_user(user);
                // send the updates through the SSE connection
                redis_to_client_stream::send_updates_to_sse(client_agent, sse_connection_to_client)
            },
        )
        .with(warp::reply::with::header("Connection", "keep-alive"))
        .recover(config::handle_errors);

    // WebSocket
    let websocket_routes = ws::extract_user_or_reject()
        .and(warp::ws::ws2())
        .and_then(move |user: user::User, ws: Ws2| {
            let token = user.access_token.clone();
            // Create a new ClientAgent
            let mut client_agent = client_agent_ws.clone_with_shared_receiver();
            // Assign that agent to generate a stream of updates for the user/timeline pair
            client_agent.init_for_user(user);
            // send the updates through the WS connection (along with the User's access_token
            // which is sent for security)
            Ok::<_, warp::Rejection>((
                ws.on_upgrade(move |socket| {
                    redis_to_client_stream::send_updates_to_ws(socket, client_agent)
                }),
                token,
            ))
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = config::cross_origin_resource_sharing();

    warp::serve(websocket_routes.or(sse_routes).with(cors)).run(*config::SERVER_ADDR);
}
