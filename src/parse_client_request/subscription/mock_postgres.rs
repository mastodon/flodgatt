//! Mock Postgres connection (for use in unit testing)
use super::{OauthScope, Subscription};
use std::collections::HashSet;

#[derive(Clone)]
pub struct PgPool;
impl PgPool {
    pub fn new() -> Self {
        Self
    }
}

pub fn select_user(
    access_token: &str,
    _pg_pool: PgPool,
) -> Result<Subscription, warp::reject::Rejection> {
    let mut user = Subscription::default();
    if access_token == "TEST_USER" {
        user.id = 1;
        user.logged_in = true;
        user.access_token = "TEST_USER".to_string();
        user.email = "user@example.com".to_string();
        user.scopes = OauthScope::from(vec![
            "read".to_string(),
            "write".to_string(),
            "follow".to_string(),
        ]);
    } else if access_token == "INVALID" {
        return Err(warp::reject::custom("Error: Invalid access token"));
    }
    Ok(user)
}

pub fn select_user_blocks(_id: i64, _pg_pool: PgPool) -> HashSet<i64> {
    HashSet::new()
}
pub fn select_domain_blocks(_pg_pool: PgPool) -> HashSet<String> {
    HashSet::new()
}

pub fn user_owns_list(user_id: i64, list_id: i64, _pg_pool: PgPool) -> bool {
    user_id == list_id
}
