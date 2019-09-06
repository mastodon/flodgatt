//! Filters for the WebSocket endpoint
use super::{
    query,
    user::{OptionalAccessToken, Scope, User},
    Query,
};
use crate::user_from_path;
use warp::{filters::BoxedFilter, path, Filter};

/// WebSocket filters
pub fn parse_query() -> BoxedFilter<(Query,)> {
    path!("api" / "v1" / "streaming")
        .and(warp::query())
        .and(query::Auth::to_filter())
        .and(query::Media::to_filter())
        .and(query::Hashtag::to_filter())
        .and(query::List::to_filter())
        .map(
            |stream: query::Stream,
             auth: query::Auth,
             media: query::Media,
             hashtag: query::Hashtag,
             list: query::List| {
                let query = Query {
                    access_token: Some(auth.access_token),
                    stream: stream.stream,
                    media: media.is_truthy(),
                    hashtag: hashtag.tag,
                    list: list.list,
                };
                query
            },
        )
        .boxed()
}

pub fn generate_timeline_and_update_user(q: Query) -> Result<(String, User), warp::Rejection> {
    let mut user = User::from_access_token_or_public_user(q.access_token).unwrap();

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
            user = user.set_filter(super::user::Filter::Notification);
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
    user.target_timeline = timeline.clone();
    Ok::<_, warp::Rejection>((timeline, user))
}

// #[cfg(false)]
// mod test {
//     use super::*;

//     struct TestUser;
//     impl TestUser {
//         fn logged_in() -> User {
//             User::from_access_token_or_reject(Some("TEST_USER".to_string())).expect("in test")
//         }
//         fn public() -> User {

//         }
//     }

//     Macro_rules! test_public_endpoint {
//         ($name:ident {
//             raw_query: $raw_query:expr,
//             parsed_query: $parsed_query:expr,
//         }) => {
//             #[test]
//             fn $name() {
//                 let (user, parsed_query, _) = warp::test::request()
//                     .path(&format!("/api/v1/streaming?{}", $raw_query))
//                     .header("connection", "upgrade")
//                     .header("upgrade", "websocket")
//                     .header("sec-websocket-version", "13")
//                     .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
//                     .filter(&extract_user_and_query())
//                     .expect("in test");

//                 assert_eq!(parsed_query.stream, $parsed_query.stream);
//                 assert_eq!(parsed_query.media, $parsed_query.media);
//                 assert_eq!(parsed_query.hashtag, $parsed_query.hashtag);
//                 assert_eq!(parsed_query.list, $parsed_query.list);
//                 assert_eq!(user, TestUser::public());
//             }
//         };
//     }
//     macro_rules! test_private_endpoint {
//         ($name:ident {
//             raw_query: $raw_query:expr,
//             parsed_query: $parsed_query:expr,
//             user: $user:expr,
//         }) => {
//             #[test]
//             fn $name() {
//                 let (user, parsed_query, _) = warp::test::request()
//                     .path(&format!(
//                         "/api/v1/streaming?access_token=TEST_USER&{}",
//                         $raw_query
//                     ))
//                     .header("connection", "upgrade")
//                     .header("upgrade", "websocket")
//                     .header("sec-websocket-version", "13")
//                     .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
//                     .filter(&extract_user_and_query())
//                     .expect("in test");

//                 assert_eq!(parsed_query.stream, $parsed_query.stream);
//                 assert_eq!(parsed_query.media, $parsed_query.media);
//                 assert_eq!(parsed_query.hashtag, $parsed_query.hashtag);
//                 assert_eq!(parsed_query.list, $parsed_query.list);
//                 assert_eq!(user, $user);
//             }
//         };
//     }

//     // macro_rules! test_private_endpoint {
//     //     ($name:ident {
//     //         endpoint: $path:expr,
//     //         $(query: $query:expr,)*
//     //         timeline: $timeline:expr,
//     //         user: $user:expr,
//     //     }) => {
//     //         #[test]
//     //         fn $name() {
//     //             let  path = format!("{}?access_token=TEST_USER", $path);
//     //             $(let path = format!("{}&{}", path, $query);)*
//     //                 let (timeline, user) = warp::test::request()
//     //                 .path(&path)
//     //                 .filter(&filter_incomming_request())
//     //                 .expect("in test");
//     //             assert_eq!(&timeline, $timeline);
//     //             assert_eq!(user, $user);
//     //             let (timeline, user) = warp::test::request()
//     //                 .path(&path)
//     //                 .header("Authorization", "Bearer: TEST_USER")
//     //                 .filter(&filter_incomming_request())
//     //                 .expect("in test");
//     //             assert_eq!(&timeline, $timeline);
//     //             assert_eq!(user, $user);
//     //         }
//     //     };
//     // }

//     //         $(query: $query:expr,)*
//     //     }) => {
//     //         #[test]
//     //         #[should_panic(expected = "Error: Missing access token")]
//     //         fn $name() {
//     //             let path = $path;
//     //             $(let path = format!("{}?{}", path, $query);)*
//     //             warp::test::request()
//     //                 .path(&path)
//     //                 .filter(&filter_incomming_request())
//     //                 .expect("in test");
//     //         }
//     //     };
//     // }

//     #[test]
//     #[should_panic(expected = "Error: Invalid access token")]
//     fn bad_auth_token() {
//         warp::test::request()
//             .path("/api/v1/streaming?stream=public&access_token=INVALID")
//             .header("connection", "upgrade")
//             .header("upgrade", "websocket")
//             .header("sec-websocket-version", "13")
//             .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
//             .filter(&extract_user_and_query())
//             .expect("to panic");
//     }

//     test_public_endpoint!(public_media_true {
//         raw_query: "stream=public&only_media=true",
//         parsed_query: Query {
//             stream: "public".to_string(),
//             media: true,
//             hashtag: String::new(),
//             list: 0,
//         },
//     });
//     test_public_endpoint!(public_media_1 {
//         raw_query: "stream=public&only_media=1",
//         parsed_query: Query {
//             stream: "public".to_string(),
//             media: true,
//             hashtag: String::new(),
//             list: 0,
//         },
//     });
//     test_private_endpoint!(user {
//         raw_query: "stream=user",
//         parsed_query: Query {
//             stream: "user".to_string(),
//             media: false,
//             hashtag: String::new(),
//             list: 0,
//         },
//         user: TestUser::logged_in(),
//     });

//     //     test_bad_auth_token_in_query!(public_media_true_bad_auth {
//     //         endpoint: "/api/v1/streaming/public",

//     //         query: "only_media=true",
//     //     });
//     //     test_bad_auth_token_in_header!(public_media_1_bad_auth {
//     //         endpoint: "/api/v1/streaming/public",
//     //         query: "only_media=1",
//     //     });

//     //     test_public_endpoint!(public_local {
//     //         endpoint: "/api/v1/streaming/public/local",
//     //         timeline: "public:local",
//     //         user: TestUser::public().set_filter(Language),
//     //     });
//     //     test_bad_auth_token_in_query!(public_local_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/public/local",
//     //     });
//     //     test_bad_auth_token_in_header!(public_local_bad_auth_in_header {
//     //         endpoint: "/api/v1/streaming/public/local",
//     //     });

//     //     test_public_endpoint!(public_local_media_true {
//     //         endpoint: "/api/v1/streaming/public/local?only_media=true",
//     //         timeline: "public:local:media",
//     //         user: TestUser::public().set_filter(Language),
//     //     });
//     //     test_public_endpoint!(public_local_media_1 {
//     //         endpoint: "/api/v1/streaming/public/local?only_media=1",
//     //         timeline: "public:local:media",
//     //         user: TestUser::public().set_filter(Language),
//     //     });
//     //     test_bad_auth_token_in_query!(public_local_media_timeline_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/public/local",
//     //         query: "only_media=1",
//     //     });
//     //     test_bad_auth_token_in_header!(public_local_media_timeline_bad_token_in_header {
//     //         endpoint: "/api/v1/streaming/public/local",
//     //         query: "only_media=true",
//     //     });

//     //     test_public_endpoint!(hashtag {
//     //         endpoint: "/api/v1/streaming/hashtag?tag=a",
//     //         timeline: "hashtag:a",
//     //         user: TestUser::public(),
//     //     });
//     //     test_bad_auth_token_in_query!(hashtag_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/hashtag",
//     //         query: "tag=a",
//     //     });
//     //     test_bad_auth_token_in_header!(hashtag_bad_auth_in_header {
//     //         endpoint: "/api/v1/streaming/hashtag",
//     //         query: "tag=a",
//     //     });

//     //     test_public_endpoint!(hashtag_local {
//     //         endpoint: "/api/v1/streaming/hashtag/local?tag=a",
//     //         timeline: "hashtag:a:local",
//     //         user: TestUser::public(),
//     //     });
//     //     test_bad_auth_token_in_query!(hashtag_local_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/hashtag/local",
//     //         query: "tag=a",
//     //     });
//     //     test_bad_auth_token_in_header!(hashtag_local_bad_auth_in_header {
//     //         endpoint: "/api/v1/streaming/hashtag/local",
//     //         query: "tag=a",
//     //     });

//     //     test_private_endpoint!(user {
//     //         endpoint: "/api/v1/streaming/user",
//     //         timeline: "1",
//     //         user: TestUser::logged_in(),
//     //     });
//     //     test_bad_auth_token_in_query!(user_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/user",
//     //     });
//     //     test_bad_auth_token_in_header!(user_bad_auth_in_header {
//     //         endpoint: "/api/v1/streaming/user",
//     //     });
//     //     test_missing_auth!(user_missing_auth_token {
//     //         endpoint: "/api/v1/streaming/user",
//     //     });

//     //     test_private_endpoint!(user_notification {
//     //         endpoint: "/api/v1/streaming/user/notification",
//     //         timeline: "1",
//     //         user: TestUser::logged_in().set_filter(Notification),
//     //     });
//     //     test_bad_auth_token_in_query!(user_notification_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/user/notification",
//     //     });
//     //     test_bad_auth_token_in_header!(user_notification_bad_auth_in_header {
//     //         endpoint: "/api/v1/streaming/user/notification",
//     //     });
//     //     test_missing_auth!(user_notification_missing_auth_token {
//     //         endpoint: "/api/v1/streaming/user/notification",
//     //     });

//     //     test_private_endpoint!(direct {
//     //         endpoint: "/api/v1/streaming/direct",
//     //         timeline: "direct:1",
//     //         user: TestUser::logged_in(),
//     //     });
//     //     test_bad_auth_token_in_query!(direct_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/direct",
//     //     });
//     //     test_bad_auth_token_in_header!(direct_bad_auth_in_header {
//     //         endpoint: "/api/v1/streaming/direct",
//     //     });
//     //     test_missing_auth!(direct_missing_auth_token {
//     //         endpoint: "/api/v1/streaming/direct",
//     //     });

//     //     test_private_endpoint!(list_valid_list {
//     //         endpoint: "/api/v1/streaming/list",
//     //         query: "list=1",
//     //         timeline: "list:1",
//     //         user: TestUser::logged_in(),
//     //     });
//     //     test_bad_auth_token_in_query!(list_bad_auth_in_query {
//     //         endpoint: "/api/v1/streaming/list",
//     //         query: "list=1",
//     //     });
//     //     test_bad_auth_token_in_header!(list_bad_auth_in_header {
//     //         endpoint: "/api/v1/streaming/list",
//     //         query: "list=1",
//     //     });
//     //     test_missing_auth!(list_missing_auth_token {
//     //         endpoint: "/api/v1/streaming/list",
//     //         query: "list=1",
//     //     });

// }
