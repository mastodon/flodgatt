use futures::{stream::Stream, Async};
use ragequit::{
    any_of, config, error,
    stream_manager::StreamManager,
    timeline,
    user::{Filter::*, User},
    ws,
};
use warp::{ws::Ws2, Filter as WarpFilter};

fn main() {
    config::logging_and_env();
    let stream_manager_sse = StreamManager::new();
    let stream_manager_ws = stream_manager_sse.clone();

    // Server Sent Events
    let sse_routes = any_of!(
        // GET /api/v1/streaming/user/notification                     [private; notification filter]
        timeline::user_notifications(),
        // GET /api/v1/streaming/user                                  [private; language filter]
        timeline::user(),
        // GET /api/v1/streaming/public/local?only_media=true          [public; language filter]
        timeline::public_local_media(),
        // GET /api/v1/streaming/public?only_media=true                [public; language filter]
        timeline::public_media(),
        // GET /api/v1/streaming/public/local                          [public; language filter]
        timeline::public_local(),
        // GET /api/v1/streaming/public                                [public; language filter]
        timeline::public(),
        // GET /api/v1/streaming/direct                                [private; *no* filter]
        timeline::direct(),
        // GET /api/v1/streaming/hashtag?tag=:hashtag                  [public; no filter]
        timeline::hashtag(),
        // GET /api/v1/streaming/hashtag/local?tag=:hashtag            [public; no filter]
        timeline::hashtag_local(),
        // GET /api/v1/streaming/list?list=:list_id                    [private; no filter]
        timeline::list()
    )
    .untuple_one()
    .and(warp::sse())
    .map(move |timeline: String, user: User, sse: warp::sse::Sse| {
        let mut stream_manager = stream_manager_sse.manage_new_timeline(&timeline, user);
        let event_stream = tokio::timer::Interval::new(
            std::time::Instant::now(),
            std::time::Duration::from_millis(100),
        )
        .filter_map(move |_| match stream_manager.poll() {
            Ok(Async::Ready(Some(json_value))) => Some((
                warp::sse::event(json_value["event"].clone().to_string()),
                warp::sse::data(json_value["payload"].clone()),
            )),
            _ => None,
        });
        sse.reply(warp::sse::keep(event_stream, None))
    })
    .with(warp::reply::with::header("Connection", "keep-alive"))
    .recover(error::handle_errors);

    // WebSocket
    let websocket_routes = ws::websocket_routes()
        .and_then(move |mut user: User, q: ws::Query, ws: Ws2| {
            let read_scope = user.scopes.clone();
            let timeline = match q.stream.as_ref() {
                // Public endpoints:
                tl @ "public" | tl @ "public:local" if q.media => format!("{}:media", tl),
                tl @ "public:media" | tl @ "public:local:media" => tl.to_string(),
                tl @ "public" | tl @ "public:local" => tl.to_string(),
                // Hashtag endpoints:
                // TODO: handle missing query
                tl @ "hashtag" | tl @ "hashtag:local" => format!("{}:{}", tl, q.hashtag),
                // Private endpoints: User
                "user" if user.logged_in && (read_scope.all || read_scope.statuses) => {
                    format!("{}", user.id)
                }
                "user:notification" if user.logged_in && (read_scope.all || read_scope.notify) => {
                    user = user.set_filter(Notification);
                    format!("{}", user.id)
                }
                // List endpoint:
                // TODO: handle missing query
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
            let token = user.access_token.clone();
            let stream_manager = stream_manager_ws.manage_new_timeline(&timeline, user);

            Ok((
                ws.on_upgrade(move |socket| ws::send_replies(socket, stream_manager)),
                token,
            ))
        })
        .map(|(reply, token)| warp::reply::with_header(reply, "sec-websocket-protocol", token));

    let cors = config::cross_origin_resource_sharing();
    let address = config::socket_address();

    warp::serve(websocket_routes.or(sse_routes).with(cors)).run(address);
}
