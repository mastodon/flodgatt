//! Postgres queries
use crate::{
    config,
    parse_client_request::user::{Scope, User},
};
use ::postgres;
use r2d2_postgres::PostgresConnectionManager;
use std::collections::HashSet;
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
    let query_rows = conn
            .query(
                "
SELECT oauth_access_tokens.resource_owner_id, users.account_id, users.chosen_languages, oauth_access_tokens.scopes
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
    if let Some(result_columns) = query_rows.get(0) {
        let mut scopes: HashSet<Scope> = result_columns
            .get::<_, String>(3)
            .split(' ')
            .filter_map(|scope| match scope {
                "read" => Some(Scope::All),
                "read:statuses" => Some(Scope::Statuses),
                "read:notifications" => Some(Scope::Notifications),
                "read:lists" => Some(Scope::Lists),
                unexpected => {
                    log::warn!("Unable to parse scope `{}`, ignoring it.", unexpected);
                    None
                }
            })
            .collect();
        if scopes.remove(&Scope::All) {
            scopes.insert(Scope::Statuses);
            scopes.insert(Scope::Notifications);
            scopes.insert(Scope::Lists);
        }
        let mut allowed_langs = HashSet::new();
        if let Ok(langs_vec) = result_columns.try_get::<_, Vec<String>>(2) {
            for lang in langs_vec {
                allowed_langs.insert(lang);
            }
        }

        Ok(User {
            id: result_columns.get(1),
            scopes,
            logged_in: true,
            allowed_langs,
            ..User::default()
        })
    } else {
        Err(warp::reject::custom("Error: Invalid access token"))
    }
}

/// Query Postgres for everyone the user has blocked or muted
///
/// **NOTE**: because we check this when the user connects, it will not include any blocks
/// the user adds until they refresh/reconnect.
pub fn select_user_blocks(user_id: i64, pg_pool: PgPool) -> HashSet<i64> {
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
/// the user adds until they refresh/reconnect.
pub fn select_domain_blocks(user_id: i64, pg_pool: PgPool) -> HashSet<String> {
    pg_pool
        .0
        .get()
        .unwrap()
        .query(
            "SELECT domain FROM account_domain_blocks WHERE account_id = $1",
            &[&user_id],
        )
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
