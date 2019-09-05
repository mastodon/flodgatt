//! Filters for all the endpoints accessible for Server Sent Event updates
use super::{
    query,
    user::{Filter::*, OptionalAccessToken, User},
};
use crate::config::CustomError;
use warp::{filters::BoxedFilter, path, Filter};

#[allow(dead_code)]
type TimelineUser = ((String, User),);

/// Helper macro to match on the first of any of the provided filters
macro_rules! any_of {
    ($filter:expr, $($other_filter:expr),*) => {
        $filter$(.or($other_filter).unify())*
    };
}

pub fn filter_incomming_request() -> BoxedFilter<(String, User)> {
    any_of!(
        path!("api" / "v1" / "streaming" / "user" / "notification")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_reject)
            .map(|user: User| (user.id.to_string(), user.set_filter(Notification))),
        // **NOTE**: This endpoint was present in the node.js server, but not in the
        // [public API docs](https://docs.joinmastodon.org/api/streaming/#get-api-v1-streaming-public-local).
        // Should it be publicly documented?
        path!("api" / "v1" / "streaming" / "user")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_reject)
            .map(|user: User| (user.id.to_string(), user)),
        path!("api" / "v1" / "streaming" / "public" / "local")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_public_user)
            .and(warp::query())
            .map(|user: User, q: query::Media| match q.only_media.as_ref() {
                "1" | "true" => ("public:local:media".to_owned(), user.set_filter(Language)),
                _ => ("public:local".to_owned(), user.set_filter(Language)),
            }),
        path!("api" / "v1" / "streaming" / "public")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_public_user)
            .and(warp::query())
            .map(|user: User, q: query::Media| match q.only_media.as_ref() {
                "1" | "true" => ("public:media".to_owned(), user.set_filter(Language)),
                _ => ("public".to_owned(), user.set_filter(Language)),
            }),
        path!("api" / "v1" / "streaming" / "public" / "local")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_public_user)
            .map(|user: User| ("public:local".to_owned(), user.set_filter(Language))),
        path!("api" / "v1" / "streaming" / "public")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_public_user)
            .map(|user: User| ("public".to_owned(), user.set_filter(Language))),
        path!("api" / "v1" / "streaming" / "direct")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_reject)
            .map(|user: User| (format!("direct:{}", user.id), user.set_filter(NoFilter))),
        // **Note**: Hashtags are *not* filtered on language, right?
        path!("api" / "v1" / "streaming" / "hashtag" / "local")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_public_user)
            .and(warp::query())
            .map(|_, q: query::Hashtag| (format!("hashtag:{}:local", q.tag), User::public())),
        path!("api" / "v1" / "streaming" / "hashtag")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_public_user)
            .and(warp::query())
            .map(|_, q: query::Hashtag| (format!("hashtag:{}", q.tag), User::public())),
        path!("api" / "v1" / "streaming" / "list")
            .and(OptionalAccessToken::from_header_or_query())
            .and_then(User::from_access_token_or_reject)
            .and(warp::query())
            .and_then(|user: User, q: query::List| {
                if user.owns_list(q.list) {
                    (Ok(q.list), Ok(user))
                } else {
                    (Err(CustomError::unauthorized_list()), Ok(user))
                }
            })
            .untuple_one()
            .map(|list: i64, user: User| (format!("list:{}", list), user.set_filter(NoFilter)))
    )
    .untuple_one()
    .boxed()
}

#[cfg(test)]
mod test {
    use super::*;

    struct TestUser;
    impl TestUser {
        fn logged_in() -> User {
            User::from_access_token_or_reject(Some("TEST_USER".to_string())).expect("in test")
        }
        fn public() -> User {
            User::from_access_token_or_public_user(None).expect("in test")
        }
    }

    macro_rules! test_public_endpoint {
        ($name:ident {
            endpoint: $path:expr,
            timeline: $timeline:expr,
            user: $user:expr,
        }) => {
            #[test]
            fn $name() {
                let (timeline, user) = warp::test::request()
                    .path($path)
                    .filter(&filter_incomming_request())
                    .expect("in test");
                assert_eq!(&timeline, $timeline);
                assert_eq!(user, $user);
            }
        };
    }

    macro_rules! test_private_endpoint {
        ($name:ident {
            endpoint: $path:expr,
            $(query: $query:expr,)*
            timeline: $timeline:expr,
            user: $user:expr,
        }) => {
            #[test]
            fn $name() {
                let  path = format!("{}?access_token=TEST_USER", $path);
                $(let path = format!("{}&{}", path, $query);)*
                    let (timeline, user) = warp::test::request()
                    .path(&path)
                    .filter(&filter_incomming_request())
                    .expect("in test");
                assert_eq!(&timeline, $timeline);
                assert_eq!(user, $user);
                let (timeline, user) = warp::test::request()
                    .path(&path)
                    .header("Authorization", "Bearer: TEST_USER")
                    .filter(&filter_incomming_request())
                    .expect("in test");
                assert_eq!(&timeline, $timeline);
                assert_eq!(user, $user);
            }
        };
    }

    macro_rules! test_bad_auth_token_in_query {
        ($name: ident {
            endpoint: $path:expr,
            $(query: $query:expr,)*
        }) => {
            #[test]
            #[should_panic(expected = "Error: Invalid access token")]

            fn $name() {
                let  path = format!("{}?access_token=INVALID", $path);
                $(let path = format!("{}&{}", path, $query);)*
                    dbg!(&path);
                    warp::test::request()
                        .path(&path)
                        .filter(&filter_incomming_request())
                        .expect("in test");
            }
        };
    }

    macro_rules! test_bad_auth_token_in_header {
        ($name: ident {
            endpoint: $path:expr,
            $(query: $query:expr,)*
        }) => {
            #[test]
            #[should_panic(expected = "Error: Invalid access token")]
            fn $name() {
                let path = $path;
                $(let path = format!("{}?{}", path, $query);)*
                    dbg!(&path);
                    warp::test::request()
                    .path(&path)
                    .header("Authorization", "Bearer: INVALID")
                    .filter(&filter_incomming_request())
                    .expect("in test");
            }
        };
    }
    macro_rules! test_missing_auth {
        ($name: ident {
            endpoint: $path:expr,
            $(query: $query:expr,)*
        }) => {
            #[test]
            #[should_panic(expected = "Error: Missing access token")]
            fn $name() {
                let path = $path;
                $(let path = format!("{}?{}", path, $query);)*
                warp::test::request()
                    .path(&path)
                    .filter(&filter_incomming_request())
                    .expect("in test");
            }
        };
    }

    test_public_endpoint!(public_media_true {
        endpoint: "/api/v1/streaming/public?only_media=true",
        timeline: "public:media",
        user: TestUser::public().set_filter(Language),
    });
    test_public_endpoint!(public_media_1 {
        endpoint: "/api/v1/streaming/public?only_media=1",
        timeline: "public:media",
        user: TestUser::public().set_filter(Language),
    });
    test_bad_auth_token_in_query!(public_media_true_bad_auth {
        endpoint: "/api/v1/streaming/public",
        query: "only_media=true",
    });
    test_bad_auth_token_in_header!(public_media_1_bad_auth {
        endpoint: "/api/v1/streaming/public",
        query: "only_media=1",
    });

    test_public_endpoint!(public_local {
        endpoint: "/api/v1/streaming/public/local",
        timeline: "public:local",
        user: TestUser::public().set_filter(Language),
    });
    test_bad_auth_token_in_query!(public_local_bad_auth_in_query {
        endpoint: "/api/v1/streaming/public/local",
    });
    test_bad_auth_token_in_header!(public_local_bad_auth_in_header {
        endpoint: "/api/v1/streaming/public/local",
    });

    test_public_endpoint!(public_local_media_true {
        endpoint: "/api/v1/streaming/public/local?only_media=true",
        timeline: "public:local:media",
        user: TestUser::public().set_filter(Language),
    });
    test_public_endpoint!(public_local_media_1 {
        endpoint: "/api/v1/streaming/public/local?only_media=1",
        timeline: "public:local:media",
        user: TestUser::public().set_filter(Language),
    });
    test_bad_auth_token_in_query!(public_local_media_timeline_bad_auth_in_query {
        endpoint: "/api/v1/streaming/public/local",
        query: "only_media=1",
    });
    test_bad_auth_token_in_header!(public_local_media_timeline_bad_token_in_header {
        endpoint: "/api/v1/streaming/public/local",
        query: "only_media=true",
    });

    test_public_endpoint!(hashtag {
        endpoint: "/api/v1/streaming/hashtag?tag=a",
        timeline: "hashtag:a",
        user: TestUser::public(),
    });
    test_bad_auth_token_in_query!(hashtag_bad_auth_in_query {
        endpoint: "/api/v1/streaming/hashtag",
        query: "tag=a",
    });
    test_bad_auth_token_in_header!(hashtag_bad_auth_in_header {
        endpoint: "/api/v1/streaming/hashtag",
        query: "tag=a",
    });

    test_public_endpoint!(hashtag_local {
        endpoint: "/api/v1/streaming/hashtag/local?tag=a",
        timeline: "hashtag:a:local",
        user: TestUser::public(),
    });
    test_bad_auth_token_in_query!(hashtag_local_bad_auth_in_query {
        endpoint: "/api/v1/streaming/hashtag/local",
        query: "tag=a",
    });
    test_bad_auth_token_in_header!(hashtag_local_bad_auth_in_header {
        endpoint: "/api/v1/streaming/hashtag/local",
        query: "tag=a",
    });

    test_private_endpoint!(user {
        endpoint: "/api/v1/streaming/user",
        timeline: "1",
        user: TestUser::logged_in(),
    });
    test_bad_auth_token_in_query!(user_bad_auth_in_query {
        endpoint: "/api/v1/streaming/user",
    });
    test_bad_auth_token_in_header!(user_bad_auth_in_header {
        endpoint: "/api/v1/streaming/user",
    });
    test_missing_auth!(user_missing_auth_token {
        endpoint: "/api/v1/streaming/user",
    });

    test_private_endpoint!(user_notification {
        endpoint: "/api/v1/streaming/user/notification",
        timeline: "1",
        user: TestUser::logged_in().set_filter(Notification),
    });
    test_bad_auth_token_in_query!(user_notification_bad_auth_in_query {
        endpoint: "/api/v1/streaming/user/notification",
    });
    test_bad_auth_token_in_header!(user_notification_bad_auth_in_header {
        endpoint: "/api/v1/streaming/user/notification",
    });
    test_missing_auth!(user_notification_missing_auth_token {
        endpoint: "/api/v1/streaming/user/notification",
    });

    test_private_endpoint!(direct {
        endpoint: "/api/v1/streaming/direct",
        timeline: "direct:1",
        user: TestUser::logged_in(),
    });
    test_bad_auth_token_in_query!(direct_bad_auth_in_query {
        endpoint: "/api/v1/streaming/direct",
    });
    test_bad_auth_token_in_header!(direct_bad_auth_in_header {
        endpoint: "/api/v1/streaming/direct",
    });
    test_missing_auth!(direct_missing_auth_token {
        endpoint: "/api/v1/streaming/direct",
    });

    test_private_endpoint!(list_valid_list {
        endpoint: "/api/v1/streaming/list",
        query: "list=1",
        timeline: "list:1",
        user: TestUser::logged_in(),
    });
    test_bad_auth_token_in_query!(list_bad_auth_in_query {
        endpoint: "/api/v1/streaming/list",
        query: "list=1",
    });
    test_bad_auth_token_in_header!(list_bad_auth_in_header {
        endpoint: "/api/v1/streaming/list",
        query: "list=1",
    });
    test_missing_auth!(list_missing_auth_token {
        endpoint: "/api/v1/streaming/list",
        query: "list=1",
    });

}
