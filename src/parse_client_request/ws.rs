//! Filters for the WebSocket endpoint
use super::{query, query::Query, user::User};
use warp::{filters::BoxedFilter, path, Filter};

/// WebSocket filters
fn parse_query() -> BoxedFilter<(Query,)> {
    path!("api" / "v1" / "streaming")
        .and(path::end())
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
                Query {
                    access_token: auth.access_token,
                    stream: stream.stream,
                    media: media.is_truthy(),
                    hashtag: hashtag.tag,
                    list: list.list,
                }
            },
        )
        .boxed()
}

pub fn extract_user_or_reject() -> BoxedFilter<(User,)> {
    parse_query()
        .and(query::OptionalAccessToken::from_ws_header())
        .and_then(Query::update_access_token)
        .and_then(User::from_query)
        .boxed()
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::parse_client_request::user::{Filter, OauthScope};

    macro_rules! test_public_endpoint {
        ($name:ident {
            endpoint: $path:expr,
            user: $user:expr,
        }) => {
            #[test]
            fn $name() {
                let user = warp::test::request()
                    .path($path)
                    .header("connection", "upgrade")
                    .header("upgrade", "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .filter(&extract_user_or_reject())
                    .expect("in test");
                assert_eq!(user, $user);
            }
        };
    }
    macro_rules! test_private_endpoint {
        ($name:ident {
            endpoint: $path:expr,
            user: $user:expr,
        }) => {
            #[test]
            fn $name() {
                let path = format!("{}&access_token=TEST_USER", $path);
                let user = warp::test::request()
                    .path(&path)
                    .header("connection", "upgrade")
                    .header("upgrade", "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .filter(&extract_user_or_reject())
                    .expect("in test");
                assert_eq!(user, $user);
            }
        };
    }
    macro_rules! test_bad_auth_token_in_query {
        ($name: ident {
            endpoint: $path:expr,

        }) => {
            #[test]
            #[should_panic(expected = "Error: Invalid access token")]

            fn $name() {
                let path = format!("{}&access_token=INVALID", $path);
                warp::test::request()
                    .path(&path)
                    .filter(&extract_user_or_reject())
                    .expect("in test");
            }
        };
    }
    macro_rules! test_missing_auth {
        ($name: ident {
            endpoint: $path:expr,
        }) => {
            #[test]
            #[should_panic(expected = "Error: Missing access token")]
            fn $name() {
                let path = $path;
                warp::test::request()
                    .path(&path)
                    .filter(&extract_user_or_reject())
                    .expect("in test");
            }
        };
    }

    test_public_endpoint!(public_media {
        endpoint: "/api/v1/streaming?stream=public:media",
        user: User {
            target_timeline: "public:media".to_string(),
            id: -1,
            access_token: "no access token".to_string(),
            langs: None,
            scopes: OauthScope {
                all: false,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: false,
            filter: Filter::Language,
        },
    });
    test_public_endpoint!(public_local {
        endpoint: "/api/v1/streaming?stream=public:local",
        user: User {
            target_timeline: "public:local".to_string(),
            id: -1,
            access_token: "no access token".to_string(),
            langs: None,
            scopes: OauthScope {
                all: false,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: false,
            filter: Filter::Language,
        },
    });
    test_public_endpoint!(public_local_media {
        endpoint: "/api/v1/streaming?stream=public:local:media",
        user: User {
            target_timeline: "public:local:media".to_string(),
            id: -1,
            access_token: "no access token".to_string(),
            langs: None,
            scopes: OauthScope {
                all: false,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: false,
            filter: Filter::Language,
        },
    });
    test_public_endpoint!(hashtag {
        endpoint: "/api/v1/streaming?stream=hashtag&tag=a",
        user: User {
            target_timeline: "hashtag:a".to_string(),
            id: -1,
            access_token: "no access token".to_string(),
            langs: None,
            scopes: OauthScope {
                all: false,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: false,
            filter: Filter::Language,
        },
    });
    test_public_endpoint!(hashtag_local {
        endpoint: "/api/v1/streaming?stream=hashtag:local&tag=a",
        user: User {
            target_timeline: "hashtag:local:a".to_string(),
            id: -1,
            access_token: "no access token".to_string(),
            langs: None,
            scopes: OauthScope {
                all: false,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: false,
            filter: Filter::Language,
        },
    });

    test_private_endpoint!(user {
        endpoint: "/api/v1/streaming?stream=user",
        user: User {
            target_timeline: "1".to_string(),
            id: 1,
            access_token: "TEST_USER".to_string(),
            langs: None,
            scopes: OauthScope {
                all: true,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: true,
            filter: Filter::NoFilter,
        },
    });
    test_private_endpoint!(user_notification {
        endpoint: "/api/v1/streaming?stream=user:notification",
        user: User {
            target_timeline: "1".to_string(),
            id: 1,
            access_token: "TEST_USER".to_string(),
            langs: None,
            scopes: OauthScope {
                all: true,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: true,
            filter: Filter::Notification,
        },
    });
    test_private_endpoint!(direct {
        endpoint: "/api/v1/streaming?stream=direct",
        user: User {
            target_timeline: "direct".to_string(),
            id: 1,
            access_token: "TEST_USER".to_string(),
            langs: None,
            scopes: OauthScope {
                all: true,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: true,
            filter: Filter::NoFilter,
        },
    });
    test_private_endpoint!(list_valid_list {
        endpoint: "/api/v1/streaming?stream=list&list=1",
        user: User {
            target_timeline: "list:1".to_string(),
            id: 1,
            access_token: "TEST_USER".to_string(),
            langs: None,
            scopes: OauthScope {
                all: true,
                statuses: false,
                notify: false,
                lists: false,
            },
            logged_in: true,
            filter: Filter::NoFilter,
        },
    });

    test_bad_auth_token_in_query!(public_media_true_bad_auth {
        endpoint: "/api/v1/streaming?stream=public:media",
    });
    test_bad_auth_token_in_query!(public_local_bad_auth_in_query {
        endpoint: "/api/v1/streaming?stream=public:local",
    });
    test_bad_auth_token_in_query!(public_local_media_timeline_bad_auth_in_query {
        endpoint: "/api/v1/streaming?stream=public:local:media",
    });
    test_bad_auth_token_in_query!(hashtag_bad_auth_in_query {
        endpoint: "/api/v1/streaming?stream=hashtag&tag=a",
    });
    test_bad_auth_token_in_query!(user_bad_auth_in_query {
        endpoint: "/api/v1/streaming?stream=user",
    });
    test_missing_auth!(user_missing_auth_token {
        endpoint: "/api/v1/streaming?stream=user",
    });
    test_bad_auth_token_in_query!(user_notification_bad_auth_in_query {
        endpoint: "/api/v1/streaming?stream=user:notification",
    });
    test_missing_auth!(user_notification_missing_auth_token {
        endpoint: "/api/v1/streaming?stream=user:notification",
    });
    test_bad_auth_token_in_query!(direct_bad_auth_in_query {
        endpoint: "/api/v1/streaming?stream=direct",
    });
    test_missing_auth!(direct_missing_auth_token {
        endpoint: "/api/v1/streaming?stream=direct",
    });
    test_bad_auth_token_in_query!(list_bad_auth_in_query {
        endpoint: "/api/v1/streaming?stream=list&list=1",
    });
    test_missing_auth!(list_missing_auth_token {
        endpoint: "/api/v1/streaming?stream=list&list=1",
    });

    #[test]
    #[should_panic(expected = "NotFound")]
    fn nonexistant_endpoint() {
        warp::test::request()
            .path("/api/v1/streaming/DOES_NOT_EXIST")
            .header("connection", "upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .filter(&extract_user_or_reject())
            .expect("in test");
    }
}
