//! Filters for all the endpoints accessible for Server Sent Event updates
use crate::query;
use crate::user::{Scope, User};
use warp::filters::BoxedFilter;
use warp::{path, Filter};

#[allow(dead_code)]
type TimelineUser = ((String, User),);

/// GET /api/v1/streaming/user
///
///
/// **private**.  Filter: `Language`
pub fn user() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "user")
        .and(path::end())
        .and(Scope::Private.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Private))
        .map(|user: User| (user.id.to_string(), user))
        .boxed()
}

/// GET /api/v1/streaming/user/notification
///
///
/// **private**.  Filter: `Notification`
///
///
/// **NOTE**: This endpoint is not included in the [public API docs](https://docs.joinmastodon.org/api/streaming/#get-api-v1-streaming-public-local).  But it was present in the JavaScript implementation, so has been included here.  Should it be publicly documented?
pub fn user_notifications() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "user" / "notification")
        .and(path::end())
        .and(Scope::Private.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Private))
        .map(|user: User| (user.id.to_string(), user.with_notification_filter()))
        .boxed()
}

/// GET /api/v1/streaming/public
///
///
/// **public**.  Filter: `Language`
pub fn public() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .and(Scope::Public.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Public))
        .map(|user: User| ("public".to_owned(), user.with_language_filter()))
        .boxed()
}

/// GET /api/v1/streaming/public?only_media=true
///
///
/// **public**.  Filter: `Language`
pub fn public_media() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "public")
        .and(path::end())
        .and(Scope::Public.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Public))
        .and(warp::query())
        .map(|user: User, q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => ("public:media".to_owned(), user.with_language_filter()),
            _ => ("public".to_owned(), user.with_language_filter()),
        })
        .boxed()
}

/// GET /api/v1/streaming/public/local
///
///
/// **public**.  Filter: `Language`
pub fn public_local() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "public" / "local")
        .and(path::end())
        .and(Scope::Public.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Public))
        .map(|user: User| ("public:local".to_owned(), user.with_language_filter()))
        .boxed()
}

/// GET /api/v1/streaming/public/local?only_media=true
///
///
/// **public**.  Filter: `Language`
pub fn public_local_media() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "public" / "local")
        .and(Scope::Public.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Public))
        .and(warp::query())
        .and(path::end())
        .map(|user: User, q: query::Media| match q.only_media.as_ref() {
            "1" | "true" => ("public:local:media".to_owned(), user.with_language_filter()),
            _ => ("public:local".to_owned(), user.with_language_filter()),
        })
        .boxed()
}

/// GET /api/v1/streaming/direct
///
///
/// **private**.  Filter: `None`
pub fn direct() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "direct")
        .and(path::end())
        .and(Scope::Private.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Private))
        .map(|user: User| (format!("direct:{}", user.id), user.with_no_filter()))
        .boxed()
}

/// GET /api/v1/streaming/hashtag?tag=:hashtag
///
///
/// **public**.  Filter: `None`
pub fn hashtag() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "hashtag")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| (format!("hashtag:{}", q.tag), User::public()))
        .boxed()
}

/// GET /api/v1/streaming/hashtag/local?tag=:hashtag
///
///
/// **public**.  Filter: `None`
pub fn hashtag_local() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "hashtag" / "local")
        .and(warp::query())
        .and(path::end())
        .map(|q: query::Hashtag| (format!("hashtag:{}:local", q.tag), User::public()))
        .boxed()
}

/// GET /api/v1/streaming/list?list=:list_id
///
///
/// **private**.  Filter: `None`
pub fn list() -> BoxedFilter<TimelineUser> {
    path!("api" / "v1" / "streaming" / "list")
        .and(Scope::Private.get_access_token())
        .and_then(|token| User::from_access_token(token, Scope::Private))
        .and(warp::query())
        .and_then(|user: User, q: query::List| (user.authorized_for_list(q.list), Ok(user)))
        .untuple_one()
        .and(path::end())
        .map(|list: i64, user: User| (format!("list:{}", list), user.with_no_filter()))
        .boxed()
}

/// Combines multiple routes with the same return type together with
/// `or()` and `unify()`
#[macro_export]
macro_rules! any_of {
    ($filter:expr, $($other_filter:expr),*) => {
        $filter$(.or($other_filter).unify())*
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user;

    #[test]
    fn user_unauthorized() {
        let value = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/user?access_token=BAD_ACCESS_TOKEN&list=1",
            ))
            .filter(&user());
        assert!(invalid_access_token(value));

        let value = warp::test::request()
            .path(&format!("/api/v1/streaming/user",))
            .filter(&user());
        assert!(no_access_token(value));
    }

    #[test]
    #[ignore]
    fn user_auth() {
        let user_id: i64 = 1;
        let access_token = get_access_token(user_id);

        // Query auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/user?access_token={}",
                access_token
            ))
            .filter(&user())
            .expect("in test");

        let expected_user =
            User::from_access_token(access_token.clone(), user::Scope::Private).expect("in test");

        assert_eq!(actual_timeline, "1");
        assert_eq!(actual_user, expected_user);

        // Header auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path("/api/v1/streaming/user")
            .header("Authorization", format!("Bearer: {}", access_token.clone()))
            .filter(&user())
            .expect("in test");

        let expected_user =
            User::from_access_token(access_token, user::Scope::Private).expect("in test");

        assert_eq!(actual_timeline, "1");
        assert_eq!(actual_user, expected_user);
    }

    #[test]
    fn user_notifications_unauthorized() {
        let value = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/user/notification?access_token=BAD_ACCESS_TOKEN",
            ))
            .filter(&user_notifications());
        assert!(invalid_access_token(value));

        let value = warp::test::request()
            .path(&format!("/api/v1/streaming/user/notification",))
            .filter(&user_notifications());
        assert!(no_access_token(value));
    }

    #[test]
    #[ignore]
    fn user_notifications_auth() {
        let user_id: i64 = 1;
        let access_token = get_access_token(user_id);

        // Query auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/user/notification?access_token={}",
                access_token
            ))
            .filter(&user_notifications())
            .expect("in test");

        let expected_user = User::from_access_token(access_token.clone(), user::Scope::Private)
            .expect("in test")
            .with_notification_filter();

        assert_eq!(actual_timeline, "1");
        assert_eq!(actual_user, expected_user);

        // Header auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path("/api/v1/streaming/user/notification")
            .header("Authorization", format!("Bearer: {}", access_token.clone()))
            .filter(&user_notifications())
            .expect("in test");

        let expected_user = User::from_access_token(access_token, user::Scope::Private)
            .expect("in test")
            .with_notification_filter();

        assert_eq!(actual_timeline, "1");
        assert_eq!(actual_user, expected_user);
    }
    #[test]
    fn public_timeline() {
        let value = warp::test::request()
            .path("/api/v1/streaming/public")
            .filter(&public())
            .expect("in test");

        assert_eq!(value.0, "public".to_string());
        assert_eq!(value.1, User::public().with_language_filter());
    }

    #[test]
    fn public_media_timeline() {
        let value = warp::test::request()
            .path("/api/v1/streaming/public?only_media=true")
            .filter(&public_media())
            .expect("in test");

        assert_eq!(value.0, "public:media".to_string());
        assert_eq!(value.1, User::public().with_language_filter());

        let value = warp::test::request()
            .path("/api/v1/streaming/public?only_media=1")
            .filter(&public_media())
            .expect("in test");

        assert_eq!(value.0, "public:media".to_string());
        assert_eq!(value.1, User::public().with_language_filter());
    }

    #[test]
    fn public_local_timeline() {
        let value = warp::test::request()
            .path("/api/v1/streaming/public/local")
            .filter(&public_local())
            .expect("in test");

        assert_eq!(value.0, "public:local".to_string());
        assert_eq!(value.1, User::public().with_language_filter());
    }

    #[test]
    fn public_local_media_timeline() {
        let value = warp::test::request()
            .path("/api/v1/streaming/public/local?only_media=true")
            .filter(&public_local_media())
            .expect("in test");

        assert_eq!(value.0, "public:local:media".to_string());
        assert_eq!(value.1, User::public().with_language_filter());

        let value = warp::test::request()
            .path("/api/v1/streaming/public/local?only_media=1")
            .filter(&public_local_media())
            .expect("in test");

        assert_eq!(value.0, "public:local:media".to_string());
        assert_eq!(value.1, User::public().with_language_filter());
    }

    #[test]
    fn direct_timeline_unauthorized() {
        let value = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/direct?access_token=BAD_ACCESS_TOKEN",
            ))
            .filter(&direct());
        assert!(invalid_access_token(value));

        let value = warp::test::request()
            .path(&format!("/api/v1/streaming/direct",))
            .filter(&direct());
        assert!(no_access_token(value));
    }

    #[test]
    #[ignore]
    fn direct_timeline_auth() {
        let user_id: i64 = 1;
        let access_token = get_access_token(user_id);

        // Query auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/direct?access_token={}",
                access_token
            ))
            .filter(&direct())
            .expect("in test");

        let expected_user =
            User::from_access_token(access_token.clone(), user::Scope::Private).expect("in test");

        assert_eq!(actual_timeline, "direct:1");
        assert_eq!(actual_user, expected_user);

        // Header auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path("/api/v1/streaming/direct")
            .header("Authorization", format!("Bearer: {}", access_token.clone()))
            .filter(&direct())
            .expect("in test");

        let expected_user =
            User::from_access_token(access_token, user::Scope::Private).expect("in test");

        assert_eq!(actual_timeline, "direct:1");
        assert_eq!(actual_user, expected_user);
    }

    #[test]
    fn hashtag_timeline() {
        let value = warp::test::request()
            .path("/api/v1/streaming/hashtag?tag=a")
            .filter(&hashtag())
            .expect("in test");

        assert_eq!(value.0, "hashtag:a".to_string());
        assert_eq!(value.1, User::public());
    }

    #[test]
    fn hashtag_timeline_local() {
        let value = warp::test::request()
            .path("/api/v1/streaming/hashtag/local?tag=a")
            .filter(&hashtag_local())
            .expect("in test");

        assert_eq!(value.0, "hashtag:a:local".to_string());
        assert_eq!(value.1, User::public());
    }

    #[test]
    #[ignore]
    fn list_timeline_auth() {
        let list_id = 1;
        let list_owner_id = get_list_owner(list_id);
        let access_token = get_access_token(list_owner_id);

        // Query Auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/list?access_token={}&list={}",
                access_token, list_id,
            ))
            .filter(&list())
            .expect("in test");

        let expected_user =
            User::from_access_token(access_token.clone(), user::Scope::Private).expect("in test");

        assert_eq!(actual_timeline, "list:1");
        assert_eq!(actual_user, expected_user);

        // Header Auth
        let (actual_timeline, actual_user) = warp::test::request()
            .path("/api/v1/streaming/list?list=1")
            .header("Authorization", format!("Bearer: {}", access_token.clone()))
            .filter(&list())
            .expect("in test");

        let expected_user =
            User::from_access_token(access_token, user::Scope::Private).expect("in test");

        assert_eq!(actual_timeline, "list:1");
        assert_eq!(actual_user, expected_user);
    }

    #[test]
    fn list_timeline_unauthorized() {
        let value = warp::test::request()
            .path(&format!(
                "/api/v1/streaming/list?access_token=BAD_ACCESS_TOKEN&list=1",
            ))
            .filter(&list());
        assert!(invalid_access_token(value));

        let value = warp::test::request()
            .path(&format!("/api/v1/streaming/list?list=1",))
            .filter(&list());
        assert!(no_access_token(value));
    }

    fn get_list_owner(list_number: i32) -> i64 {
        let list_number: i64 = list_number.into();
        let conn = user::connect_to_postgres();
        let rows = &conn
            .query(
                "SELECT id, account_id FROM lists WHERE id = $1 LIMIT 1",
                &[&list_number],
            )
            .expect("in test");

        assert_eq!(
            rows.len(),
            1,
            "Test database must contain at least one user with a list to run this test."
        );

        rows.get(0).get(1)
    }
    fn get_access_token(user_id: i64) -> String {
        let conn = user::connect_to_postgres();
        let rows = &conn
            .query(
                "SELECT token FROM oauth_access_tokens WHERE resource_owner_id = $1",
                &[&user_id],
            )
            .expect("Can get access token from id");
        rows.get(0).get(0)
    }
    fn invalid_access_token(value: Result<(String, User), warp::reject::Rejection>) -> bool {
        match value {
            Err(error) => match error.cause() {
                Some(c) if format!("{:?}", c) == "StringError(\"Error: Invalid access token\")" => {
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn no_access_token(value: Result<(String, User), warp::reject::Rejection>) -> bool {
        match value {
            Err(error) => match error.cause() {
                Some(c) if format!("{:?}", c) == "MissingHeader(\"authorization\")" => true,
                _ => false,
            },
            _ => false,
        }
    }
}
