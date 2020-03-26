//! Postgres queries
use crate::{
    config,
    parse_client_request::subscription::{Scope, UserData},
};
use ::postgres;
use r2d2_postgres::PostgresConnectionManager;
use std::collections::HashSet;
use warp::reject::Rejection;

#[derive(Clone, Debug)]
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

pub fn select_user(token: &str, pool: PgPool) -> Result<UserData, Rejection> {
    let mut conn = pool.0.get().unwrap();
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
                &[&token.to_owned()],
            )
        .expect("Hard-coded query will return Some([0 or more rows])");
    if let Some(result_columns) = query_rows.get(0) {
        let id = result_columns.get(1);
        let allowed_langs = result_columns
            .try_get::<_, Vec<_>>(2)
            .unwrap_or_else(|_| Vec::new())
            .into_iter()
            .collect();
        let mut scopes: HashSet<Scope> = result_columns
            .get::<_, String>(3)
            .split(' ')
            .filter_map(|scope| match scope {
                "read" => Some(Scope::Read),
                "read:statuses" => Some(Scope::Statuses),
                "read:notifications" => Some(Scope::Notifications),
                "read:lists" => Some(Scope::Lists),
                "write" | "follow" => None, // ignore write scopes
                unexpected => {
                    log::warn!("Ignoring unknown scope `{}`", unexpected);
                    None
                }
            })
            .collect();
        // We don't need to separately track read auth - it's just all three others
        if scopes.remove(&Scope::Read) {
            scopes.insert(Scope::Statuses);
            scopes.insert(Scope::Notifications);
            scopes.insert(Scope::Lists);
        }

        Ok(UserData {
            id,
            allowed_langs,
            scopes,
        })
    } else {
        Err(warp::reject::custom("Error: Invalid access token"))
    }
}

pub fn select_hashtag_id(tag_name: &String, pg_pool: PgPool) -> Result<i64, Rejection> {
    let mut conn = pg_pool.0.get().unwrap();
    let rows = &conn
        .query(
            "
SELECT id
FROM tags
WHERE name = $1
LIMIT 1",
            &[&tag_name],
        )
        .expect("Hard-coded query will return Some([0 or more rows])");

    match rows.get(0) {
        Some(row) => Ok(row.get(0)),
        None => Err(warp::reject::custom("Error: Hashtag does not exist.")),
    }
}
pub fn select_hashtag_name(tag_id: &i64, pg_pool: PgPool) -> Result<String, Rejection> {
    let mut conn = pg_pool.0.get().unwrap();
    // For the Postgres query, `id` = list number; `account_id` = user.id
    let rows = &conn
        .query(
            "
SELECT name
FROM tags
WHERE id = $1
LIMIT 1",
            &[&tag_id],
        )
        .expect("Hard-coded query will return Some([0 or more rows])");

    match rows.get(0) {
        Some(row) => Ok(row.get(0)),
        None => Err(warp::reject::custom("Error: Hashtag does not exist.")),
    }
}

/// Query Postgres for everyone the user has blocked or muted
///
/// **NOTE**: because we check this when the user connects, it will not include any blocks
/// the user adds until they refresh/reconnect.
pub fn select_blocked_users(user_id: i64, pg_pool: PgPool) -> HashSet<i64> {
    //     "
    // SELECT
    //    1
    //    FROM blocks
    //    WHERE (account_id = $1 AND target_account_id IN (${placeholders(targetAccountIds, 2)}))
    //    OR (account_id = $2 AND target_account_id = $1)
    // UNION SELECT
    //    1
    //    FROM mutes
    //    WHERE account_id = $1 AND target_account_id IN (${placeholders(targetAccountIds, 2)})`
    // , [req.accountId, unpackedPayload.account.id].concat(targetAccountIds)),`"
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
/// Query Postgres for everyone who has blocked the user
///
/// **NOTE**: because we check this when the user connects, it will not include any blocks
/// the user adds until they refresh/reconnect.
pub fn select_blocking_users(user_id: i64, pg_pool: PgPool) -> HashSet<i64> {
    pg_pool
        .0
        .get()
        .unwrap()
        .query(
            "
SELECT account_id
  FROM blocks
  WHERE target_account_id = $1",
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
pub fn select_blocked_domains(user_id: i64, pg_pool: PgPool) -> HashSet<String> {
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
