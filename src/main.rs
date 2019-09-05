use log::{log_enabled, Level};
use ragequit::{
    config,
    parse_client_request::{sse, user, ws},
    redis_to_client_stream,
    redis_to_client_stream::ClientAgent,
};
use warp::{ws::Ws2, Filter as WarpFilter};

fn main() {
    config::logging_and_env();
    let client_agent_sse = ClientAgent::blank();
    let client_agent_ws = client_agent_sse.clone_with_shared_receiver();

    if log_enabled!(Level::Warn) {
        println!("Streaming server initialized and ready to accept connections");
    };

    // Server Sent Events
    //
    // For SSE, the API requires users to use different endpoints, so we first filter based on
    // the endpoint.  Using that endpoint determine the `timeline` the user is requesting,
    // the scope for that `timeline`, and authenticate the `User` if they provided a token.
    let sse_routes = sse::filter_incomming_request()
        .and(warp::sse())
        .map(
            move |timeline: String, user: user::User, sse_connection_to_client: warp::sse::Sse| {
                // Create a new ClientAgent
                let mut client_agent = client_agent_sse.clone_with_shared_receiver();
                // Assign that agent to generate a stream of updates for the user/timeline pair
                client_agent.init_for_user(&timeline, user);
                // send the updates through the SSE connection
                redis_to_client_stream::send_updates_to_sse(client_agent, sse_connection_to_client)
            },
        )
        .with(warp::reply::with::header("Connection", "keep-alive"))
        .recover(config::handle_errors);

    // WebSocket
    //
    // For WS, the API specifies a single endpoint, so we extract the User/timeline pair
    // directy from the query
    let websocket_routes = ws::extract_user_and_query()
        .and_then(move |mut user: user::User, q: ws::Query, ws: Ws2| {
            let token = user.access_token.clone();
            let read_scope = user.scopes.clone();

            let timeline = match q.stream.as_ref() {
                // Public endpoints:
                tl @ "public" | tl @ "public:local" if q.media => format!("{}:media", tl),
                tl @ "public:media" | tl @ "public:local:media" => tl.to_string(),
                tl @ "public" | tl @ "public:local" => tl.to_string(),
                // Hashtag endpoints:
                tl @ "hashtag" | tl @ "hashtag:local" => format!("{}:{}", tl, q.hashtag),
                // Private endpoints: User
                "user" if user.logged_in && (read_scope.all || read_scope.statuses) => {
                    format!("{}", user.id)
                }
                "user:notification" if user.logged_in && (read_scope.all || read_scope.notify) => {
                    user = user.set_filter(user::Filter::Notification);
                    format!("{}", user.id)
                }
                // List endpoint:
                "list" if user.owns_list(q.list) && (read_scope.all || read_scope.lists) => {
                    format!("list:{}", q.list)
                }
                // Direct endpoint:
                "direct" if user.logged_in && (read_scope.all || read_scope.statuses) => {
                    "direct".to_string()
                }
                // Reject unathorized access attempts for private endpoints
                "user" | "user:notification" | "direct" | "list" => {
                    return Err(warp::reject::custom("Error: Invalid Access Token"))
                }
                // Other endpoints don't exist:
                _ => return Err(warp::reject::custom("Error: Nonexistent WebSocket query")),
            };

            // Create a new ClientAgent
            let mut client_agent = client_agent_ws.clone_with_shared_receiver();
            // Assign that agent to generate a stream of updates for the user/timeline pair
            client_agent.init_for_user(&timeline, user);
            // send the updates through the WS connection (along with the User's access_token
            // which is sent for security)
            Ok((
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
