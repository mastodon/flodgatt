//! Filters for all the endpoints accessible for Server Sent Event updates
use super::{
    query::{self, Query},
    subscription::{PgPool, Subscription},
};
use warp::{filters::BoxedFilter, path, Filter};
#[allow(dead_code)]
type TimelineUser = ((String, Subscription),);

/// Helper macro to match on the first of any of the provided filters
macro_rules! any_of {
    ($filter:expr, $($other_filter:expr),*) => {
        $filter$(.or($other_filter).unify())*.boxed()
    };
}

macro_rules! parse_query {
    (path => $start:tt $(/ $next:tt)*
     endpoint => $endpoint:expr) => {
        path!($start $(/ $next)*)
            .and(query::Auth::to_filter())
            .and(query::Media::to_filter())
            .and(query::Hashtag::to_filter())
            .and(query::List::to_filter())
            .map(
                |auth: query::Auth,
                 media: query::Media,
                 hashtag: query::Hashtag,
                 list: query::List| {
                    Query {
                        access_token: auth.access_token,
                        stream: $endpoint.to_string(),
                        media: media.is_truthy(),
                        hashtag: hashtag.tag,
                        list: list.list,
                    }
                 },
            )
            .boxed()
    };
}
pub fn extract_user_or_reject(
    pg_pool: PgPool,
    whitelist_mode: bool,
) -> BoxedFilter<(Subscription,)> {
    any_of!(
        parse_query!(
            path => "api" / "v1" / "streaming" / "user" / "notification"
            endpoint => "user:notification" ),
        parse_query!(
            path => "api" / "v1" / "streaming" / "user"
            endpoint => "user"),
        parse_query!(
            path => "api" / "v1" / "streaming" / "public" / "local"
            endpoint => "public:local"),
        parse_query!(
            path => "api" / "v1" / "streaming" / "public"
            endpoint => "public"),
        parse_query!(
            path => "api" / "v1" / "streaming" / "direct"
            endpoint => "direct"),
        parse_query!(path => "api" / "v1" / "streaming" / "hashtag" / "local"
                     endpoint => "hashtag:local"),
        parse_query!(path => "api" / "v1" / "streaming" / "hashtag"
                     endpoint => "hashtag"),
        parse_query!(path => "api" / "v1" / "streaming" / "list"
                endpoint => "list")
    )
    // because SSE requests place their `access_token` in the header instead of in a query
    // parameter, we need to update our Query if the header has a token
    .and(query::OptionalAccessToken::from_sse_header())
    .and_then(Query::update_access_token)
    .and_then(move |q| Subscription::from_query(q, pg_pool.clone(), whitelist_mode))
    .boxed()
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::parse_client_request::user::{Blocks, Filter, OauthScope, PgPool};

//     macro_rules! test_public_endpoint {
//         ($name:ident {
//             endpoint: $path:expr,
//             user: $user:expr,
//         }) => {
//             #[test]
//             fn $name() {
//                 let mock_pg_pool = PgPool::new();
//                 let user = warp::test::request()
//                     .path($path)
//                     .filter(&extract_user_or_reject(mock_pg_pool))
//                     .expect("in test");
//                 assert_eq!(user, $user);
//             }
//         };
//     }
//     macro_rules! test_private_endpoint {
//         ($name:ident {
//             endpoint: $path:expr,
//             $(query: $query:expr,)*
//             user: $user:expr,
//         }) => {
//             #[test]
//             fn $name() {
//                 let  path = format!("{}?access_token=TEST_USER", $path);
//                 let mock_pg_pool = PgPool::new();
//                 $(let path = format!("{}&{}", path, $query);)*
//                     let  user = warp::test::request()
//                     .path(&path)
//                     .filter(&extract_user_or_reject(mock_pg_pool.clone()))
//                     .expect("in test");
//                 assert_eq!(user, $user);
//                 let user = warp::test::request()
//                     .path(&path)
//                     .header("Authorization", "Bearer: TEST_USER")
//                     .filter(&extract_user_or_reject(mock_pg_pool))
//                     .expect("in test");
//                 assert_eq!(user, $user);
//             }
//         };
//     }
//     macro_rules! test_bad_auth_token_in_query {
//         ($name: ident {
//             endpoint: $path:expr,
//             $(query: $query:expr,)*
//         }) => {
//             #[test]
//             #[should_panic(expected = "Error: Invalid access token")]
//             fn $name() {
//                 let  path = format!("{}?access_token=INVALID", $path);
//                 $(let path = format!("{}&{}", path, $query);)*
//                 let mock_pg_pool = PgPool::new();
//                 warp::test::request()
//                     .path(&path)
//                     .filter(&extract_user_or_reject(mock_pg_pool))
//                     .expect("in test");
//             }
//         };
//     }
//     macro_rules! test_bad_auth_token_in_header {
//         ($name: ident {
//             endpoint: $path:expr,
//             $(query: $query:expr,)*
//         }) => {
//             #[test]
//             #[should_panic(expected = "Error: Invalid access token")]
//             fn $name() {
//                 let path = $path;
//                 $(let path = format!("{}?{}", path, $query);)*

//                 let mock_pg_pool = PgPool::new();
//                 warp::test::request()
//                     .path(&path)
//                     .header("Authorization", "Bearer: INVALID")
//                     .filter(&extract_user_or_reject(mock_pg_pool))
//                     .expect("in test");
//             }
//         };
//     }
//     macro_rules! test_missing_auth {
//         ($name: ident {
//             endpoint: $path:expr,
//             $(query: $query:expr,)*
//         }) => {
//             #[test]
//             #[should_panic(expected = "Error: Missing access token")]
//             fn $name() {
//                 let path = $path;
//                 $(let path = format!("{}?{}", path, $query);)*
//                 let mock_pg_pool = PgPool::new();
//                 warp::test::request()
//                     .path(&path)
//                     .filter(&extract_user_or_reject(mock_pg_pool))
//                     .expect("in test");
//             }
//         };
//     }

//     test_public_endpoint!(public_media_true {
//         endpoint: "/api/v1/streaming/public?only_media=true",
//         user: Subscription {
//             timeline: "public:media".to_string(),
//             id: -1,
//             email: "".to_string(),
//             access_token: "".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: false,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: false,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Language,
//         },
//     });
//     test_public_endpoint!(public_media_1 {
//         endpoint: "/api/v1/streaming/public?only_media=1",
//         user: Subscription {
//             timeline: "public:media".to_string(),
//             id: -1,
//             email: "".to_string(),
//             access_token: "".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: false,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: false,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Language,
//         },
//     });
//     test_public_endpoint!(public_local {
//         endpoint: "/api/v1/streaming/public/local",
//         user: Subscription {
//             timeline: "public:local".to_string(),
//             id: -1,
//             email: "".to_string(),
//             access_token: "".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: false,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: false,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Language,
//         },
//     });
//     test_public_endpoint!(public_local_media_true {
//         endpoint: "/api/v1/streaming/public/local?only_media=true",
//         user: Subscription {
//             timeline: "public:local:media".to_string(),
//             id: -1,
//             email: "".to_string(),
//             access_token: "".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: false,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: false,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Language,
//         },
//     });
//     test_public_endpoint!(public_local_media_1 {
//         endpoint: "/api/v1/streaming/public/local?only_media=1",
//         user: Subscription {
//             timeline: "public:local:media".to_string(),
//             id: -1,
//             email: "".to_string(),
//             access_token: "".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: false,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: false,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Language,
//         },
//     });
//     test_public_endpoint!(hashtag {
//         endpoint: "/api/v1/streaming/hashtag?tag=a",
//         user: Subscription {
//             timeline: "hashtag:a".to_string(),
//             id: -1,
//             email: "".to_string(),
//             access_token: "".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: false,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: false,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Language,
//         },
//     });
//     test_public_endpoint!(hashtag_local {
//         endpoint: "/api/v1/streaming/hashtag/local?tag=a",
//         user: Subscription {
//             timeline: "hashtag:local:a".to_string(),
//             id: -1,
//             email: "".to_string(),
//             access_token: "".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: false,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: false,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Language,
//         },
//     });

//     test_private_endpoint!(user {
//         endpoint: "/api/v1/streaming/user",
//         user: Subscription {
//             timeline: "1".to_string(),
//             id: 1,
//             email: "user@example.com".to_string(),
//             access_token: "TEST_USER".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: true,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: true,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::NoFilter,
//         },
//     });
//     test_private_endpoint!(user_notification {
//         endpoint: "/api/v1/streaming/user/notification",
//         user: Subscription {
//             timeline: "1".to_string(),
//             id: 1,
//             email: "user@example.com".to_string(),
//             access_token: "TEST_USER".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: true,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: true,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::Notification,
//         },
//     });
//     test_private_endpoint!(direct {
//         endpoint: "/api/v1/streaming/direct",
//         user: Subscription {
//             timeline: "direct".to_string(),
//             id: 1,
//             email: "user@example.com".to_string(),
//             access_token: "TEST_USER".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: true,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: true,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::NoFilter,
//         },
//     });

//     test_private_endpoint!(list_valid_list {
//         endpoint: "/api/v1/streaming/list",
//         query: "list=1",
//         user: Subscription {
//             timeline: "list:1".to_string(),
//             id: 1,
//             email: "user@example.com".to_string(),
//             access_token: "TEST_USER".to_string(),
//             langs: None,
//             scopes: OauthScope {
//                 all: true,
//                 statuses: false,
//                 notify: false,
//                 lists: false,
//             },
//             logged_in: true,
//             blocks: Blocks::default(),
//             allowed_langs: Filter::NoFilter,
//         },
//     });
//     test_bad_auth_token_in_query!(public_media_true_bad_auth {
//         endpoint: "/api/v1/streaming/public",
//         query: "only_media=true",
//     });
//     test_bad_auth_token_in_header!(public_media_1_bad_auth {
//         endpoint: "/api/v1/streaming/public",
//         query: "only_media=1",
//     });
//     test_bad_auth_token_in_query!(public_local_bad_auth_in_query {
//         endpoint: "/api/v1/streaming/public/local",
//     });
//     test_bad_auth_token_in_header!(public_local_bad_auth_in_header {
//         endpoint: "/api/v1/streaming/public/local",
//     });
//     test_bad_auth_token_in_query!(public_local_media_timeline_bad_auth_in_query {
//         endpoint: "/api/v1/streaming/public/local",
//         query: "only_media=1",
//     });
//     test_bad_auth_token_in_header!(public_local_media_timeline_bad_token_in_header {
//         endpoint: "/api/v1/streaming/public/local",
//         query: "only_media=true",
//     });
//     test_bad_auth_token_in_query!(hashtag_bad_auth_in_query {
//         endpoint: "/api/v1/streaming/hashtag",
//         query: "tag=a",
//     });
//     test_bad_auth_token_in_header!(hashtag_bad_auth_in_header {
//         endpoint: "/api/v1/streaming/hashtag",
//         query: "tag=a",
//     });
//     test_bad_auth_token_in_query!(user_bad_auth_in_query {
//         endpoint: "/api/v1/streaming/user",
//     });
//     test_bad_auth_token_in_header!(user_bad_auth_in_header {
//         endpoint: "/api/v1/streaming/user",
//     });
//     test_missing_auth!(user_missing_auth_token {
//         endpoint: "/api/v1/streaming/user",
//     });
//     test_bad_auth_token_in_query!(user_notification_bad_auth_in_query {
//         endpoint: "/api/v1/streaming/user/notification",
//     });
//     test_bad_auth_token_in_header!(user_notification_bad_auth_in_header {
//         endpoint: "/api/v1/streaming/user/notification",
//     });
//     test_missing_auth!(user_notification_missing_auth_token {
//         endpoint: "/api/v1/streaming/user/notification",
//     });
//     test_bad_auth_token_in_query!(direct_bad_auth_in_query {
//         endpoint: "/api/v1/streaming/direct",
//     });
//     test_bad_auth_token_in_header!(direct_bad_auth_in_header {
//         endpoint: "/api/v1/streaming/direct",
//     });
//     test_missing_auth!(direct_missing_auth_token {
//         endpoint: "/api/v1/streaming/direct",
//     });
//     test_bad_auth_token_in_query!(list_bad_auth_in_query {
//         endpoint: "/api/v1/streaming/list",
//         query: "list=1",
//     });
//     test_bad_auth_token_in_header!(list_bad_auth_in_header {
//         endpoint: "/api/v1/streaming/list",
//         query: "list=1",
//     });
//     test_missing_auth!(list_missing_auth_token {
//         endpoint: "/api/v1/streaming/list",
//         query: "list=1",
//     });

//     #[test]
//     #[should_panic(expected = "NotFound")]
//     fn nonexistant_endpoint() {
//         let mock_pg_pool = PgPool::new();
//         warp::test::request()
//             .path("/api/v1/streaming/DOES_NOT_EXIST")
//             .filter(&extract_user_or_reject(mock_pg_pool))
//             .expect("in test");
//     }
// }
