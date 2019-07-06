use ragequit::{
    config,
    timeline::*,
    user::{Filter::*, Scope, User},
};

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
        User::from_access_token(access_token.clone(), Scope::Private).expect("in test");

    assert_eq!(actual_timeline, "1");
    assert_eq!(actual_user, expected_user);

    // Header auth
    let (actual_timeline, actual_user) = warp::test::request()
        .path("/api/v1/streaming/user")
        .header("Authorization", format!("Bearer: {}", access_token.clone()))
        .filter(&user())
        .expect("in test");

    let expected_user = User::from_access_token(access_token, Scope::Private).expect("in test");

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

    let expected_user = User::from_access_token(access_token.clone(), Scope::Private)
        .expect("in test")
        .set_filter(Notification);

    assert_eq!(actual_timeline, "1");
    assert_eq!(actual_user, expected_user);

    // Header auth
    let (actual_timeline, actual_user) = warp::test::request()
        .path("/api/v1/streaming/user/notification")
        .header("Authorization", format!("Bearer: {}", access_token.clone()))
        .filter(&user_notifications())
        .expect("in test");

    let expected_user = User::from_access_token(access_token, Scope::Private)
        .expect("in test")
        .set_filter(Notification);

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
    assert_eq!(value.1, User::public().set_filter(Language));
}

#[test]
fn public_media_timeline() {
    let value = warp::test::request()
        .path("/api/v1/streaming/public?only_media=true")
        .filter(&public_media())
        .expect("in test");

    assert_eq!(value.0, "public:media".to_string());
    assert_eq!(value.1, User::public().set_filter(Language));

    let value = warp::test::request()
        .path("/api/v1/streaming/public?only_media=1")
        .filter(&public_media())
        .expect("in test");

    assert_eq!(value.0, "public:media".to_string());
    assert_eq!(value.1, User::public().set_filter(Language));
}

#[test]
fn public_local_timeline() {
    let value = warp::test::request()
        .path("/api/v1/streaming/public/local")
        .filter(&public_local())
        .expect("in test");

    assert_eq!(value.0, "public:local".to_string());
    assert_eq!(value.1, User::public().set_filter(Language));
}

#[test]
fn public_local_media_timeline() {
    let value = warp::test::request()
        .path("/api/v1/streaming/public/local?only_media=true")
        .filter(&public_local_media())
        .expect("in test");

    assert_eq!(value.0, "public:local:media".to_string());
    assert_eq!(value.1, User::public().set_filter(Language));

    let value = warp::test::request()
        .path("/api/v1/streaming/public/local?only_media=1")
        .filter(&public_local_media())
        .expect("in test");

    assert_eq!(value.0, "public:local:media".to_string());
    assert_eq!(value.1, User::public().set_filter(Language));
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
        User::from_access_token(access_token.clone(), Scope::Private).expect("in test");

    assert_eq!(actual_timeline, "direct:1");
    assert_eq!(actual_user, expected_user);

    // Header auth
    let (actual_timeline, actual_user) = warp::test::request()
        .path("/api/v1/streaming/direct")
        .header("Authorization", format!("Bearer: {}", access_token.clone()))
        .filter(&direct())
        .expect("in test");

    let expected_user = User::from_access_token(access_token, Scope::Private).expect("in test");

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
        User::from_access_token(access_token.clone(), Scope::Private).expect("in test");

    assert_eq!(actual_timeline, "list:1");
    assert_eq!(actual_user, expected_user);

    // Header Auth
    let (actual_timeline, actual_user) = warp::test::request()
        .path("/api/v1/streaming/list?list=1")
        .header("Authorization", format!("Bearer: {}", access_token.clone()))
        .filter(&list())
        .expect("in test");

    let expected_user = User::from_access_token(access_token, Scope::Private).expect("in test");

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

// Helper functions for tests
fn get_list_owner(list_number: i32) -> i64 {
    let list_number: i64 = list_number.into();
    let conn = config::postgres();
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
    let conn = config::postgres();
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
            Some(c) if format!("{:?}", c) == "StringError(\"Error: Invalid access token\")" => true,
            _ => false,
        },
        _ => false,
    }
}

fn no_access_token(value: Result<(String, User), warp::reject::Rejection>) -> bool {
    match value {
        Err(error) => match error.cause() {
            // The cause could validly be any of these, depending on the order they're checked
            // (It would pass with just one, so the last one it doesn't have is "the" cause)
            Some(c) if format!("{:?}", c) == "MissingHeader(\"authorization\")" => true,
            Some(c) if format!("{:?}", c) == "InvalidQuery" => true,
            Some(c) if format!("{:?}", c) == "MissingHeader(\"Sec-WebSocket-Protocol\")" => true,
            _ => false,
        },
        _ => false,
    }
}
