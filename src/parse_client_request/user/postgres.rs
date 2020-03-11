//! Postgres queries
use crate::{
    config,
    parse_client_request::user::{OauthScope, User},
};
use ::postgres;
use r2d2_postgres::PostgresConnectionManager;
use warp::reject::Rejection;

#[derive(Clone)]
pub struct PgPool(pub r2d2::Pool<PostgresConnectionManager<postgres::NoTls>>);
impl PgPool {
    pub fn new(pg_cfg: config::PostgresConfig) -> Self {
        let mut cfg = postgres::Config::new();
        cfg.user(&pg_cfg.user)
            .host(&*pg_cfg.host.to_string())
            .port(*pg_cfg.port)
            .dbname(&pg_cfg.database);
        if let Some(password) = &*pg_cfg.password {
            cfg.password(password);
        };

        let manager = PostgresConnectionManager::new(cfg, postgres::NoTls);
        let pool = r2d2::Pool::builder()
            .max_size(10)
            .build(manager)
            .expect("Can connect to local postgres");
        Self(pool)
    }
}

/// Build a user based on the result of querying Postgres with the access token
///
/// This does _not_ set the timeline, filter, or blocks fields.  Use the various `User`
/// methods to do so.  In general, this function shouldn't be needed outside `User`.
pub fn select_user(access_token: &str, pg_pool: PgPool) -> Result<User, Rejection> {
    let mut conn = pg_pool.0.get().unwrap();
    let query_result = conn
            .query(
                "
SELECT oauth_access_tokens.resource_owner_id, users.account_id, users.email, users.chosen_languages, oauth_access_tokens.scopes
FROM
oauth_access_tokens
INNER JOIN users ON
oauth_access_tokens.resource_owner_id = users.id
WHERE oauth_access_tokens.token = $1
AND oauth_access_tokens.revoked_at IS NULL
LIMIT 1",
                &[&access_token.to_owned()],
            )
            .expect("Hard-coded query will return Some([0 or more rows])");
    if query_result.is_empty() {
        Err(warp::reject::custom("Error: Invalid access token"))
    } else {
        let only_row: &postgres::Row = query_result.get(0).unwrap();
        let scope_vec: Vec<String> = only_row
            .get::<_, String>(4)
            .split(' ')
            .map(|s| s.to_owned())
            .collect();
        Ok(User {
            id: only_row.get(1),
            email: only_row.get(2),
            logged_in: true,
            scopes: OauthScope::from(scope_vec),
            langs: only_row.get(3),
            ..User::default()
        })
    }
}

#[cfg(test)]
pub fn query_for_user_data(access_token: &str) -> (i64, Option<Vec<String>>, Vec<String>) {
    let (user_id, lang, scopes) = if access_token == "TEST_USER" {
        (
            1,
            None,
            vec![
                "read".to_string(),
                "write".to_string(),
                "follow".to_string(),
            ],
        )
    } else {
        (-1, None, Vec::new())
    };
    (user_id, lang, scopes)
}

/// Query Postgres for everyone the user has blocked or muted
///
/// **NOTE**: because we check this when the user connects, it will not include any blocks
/// the user adds until they refresh/reconnect.
pub fn select_user_blocks(user_id: i64, pg_pool: PgPool) -> Vec<i64> {
    pg_pool
        .0
        .get()
        .unwrap()
        .query(
            "
SELECT target_account_id
  FROM blocks
  WHERE account_id = $1
UNION SELECT target_account_id
  FROM mutes
  WHERE account_id = $1",
            &[&user_id],
        )
        .expect("Hard-coded query will return Some([0 or more rows])")
        .iter()
        .map(|row| row.get(0))
        .collect()
}

/// Query Postgres for all current domain blocks
///
/// **NOTE**: because we check this when the user connects, it will not include any blocks
/// the user adds until they refresh/reconnect.  Additionally, we are querying it once per
/// user, even though it is constant for all users (at any given time).
pub fn select_domain_blocks(pg_pool: PgPool) -> Vec<String> {
    pg_pool
        .0
        .get()
        .unwrap()
        .query("SELECT domain FROM domain_blocks", &[])
        .expect("Hard-coded query will return Some([0 or more rows])")
        .iter()
        .map(|row| row.get(0))
        .collect()
}

/// Test whether a user owns a list
pub fn user_owns_list(user_id: i64, list_id: i64, pg_pool: PgPool) -> bool {
    let mut conn = pg_pool.0.get().unwrap();
    // For the Postgres query, `id` = list number; `account_id` = user.id
    let rows = &conn
        .query(
            "
SELECT id, account_id
FROM lists
WHERE id = $1
LIMIT 1",
            &[&list_id],
        )
        .expect("Hard-coded query will return Some([0 or more rows])");

    match rows.get(0) {
        None => false,
        Some(row) => {
            let list_owner_id: i64 = row.get(1);
            list_owner_id == user_id
        }
    }
}
