//! Postgres queries
use crate::config;
use crate::messages::Id;
use crate::request::timeline::{Scope, UserData};

use ::postgres;
use hashbrown::HashSet;
use r2d2_postgres::PostgresConnectionManager;
use std::convert::TryFrom;
use warp::reject::Rejection;

#[derive(Clone, Debug)]
pub struct PgPool {
    pub conn: r2d2::Pool<PostgresConnectionManager<postgres::NoTls>>,
    whitelist_mode: bool,
}

impl PgPool {
    pub fn new(pg_cfg: config::Postgres, whitelist_mode: bool) -> Self {
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
        Self {
            conn: pool,
            whitelist_mode,
        }
    }

    pub fn select_user(self, token: &Option<String>) -> Result<UserData, Rejection> {
        let mut conn = self.conn.get().unwrap();
        if let Some(token) = token {
            let query_rows = conn
                .query("
SELECT oauth_access_tokens.resource_owner_id, users.account_id, users.chosen_languages, oauth_access_tokens.scopes
  FROM oauth_access_tokens
INNER JOIN users ON oauth_access_tokens.resource_owner_id = users.id
  WHERE oauth_access_tokens.token = $1 AND oauth_access_tokens.revoked_at IS NULL
LIMIT 1",
                &[&token.to_owned()],
            )
        .expect("Hard-coded query will return Some([0 or more rows])");
            if let Some(result_columns) = query_rows.get(0) {
                let id = Id(result_columns.get(1));
                let allowed_langs = result_columns
                    .try_get::<_, Vec<_>>(2)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();

                let mut scopes: HashSet<Scope> = result_columns
                    .get::<_, String>(3)
                    .split(' ')
                    .filter_map(|scope| Scope::try_from(scope).ok())
                    .collect();
                // We don't need to separately track read auth - it's just all three others
                if scopes.contains(&Scope::Read) {
                    scopes = vec![Scope::Statuses, Scope::Notifications, Scope::Lists]
                        .into_iter()
                        .collect()
                }

                Ok(UserData {
                    id,
                    allowed_langs,
                    scopes,
                })
            } else {
                Err(warp::reject::custom("Error: Invalid access token"))
            }
        } else if self.whitelist_mode {
            Err(warp::reject::custom("Error: Invalid access token"))
        } else {
            Ok(UserData::public())
        }
    }

    pub fn select_hashtag_id(self, tag_name: &str) -> Result<i64, Rejection> {
        let mut conn = self.conn.get().expect("TODO");
        conn.query("SELECT id FROM tags WHERE name = $1 LIMIT 1", &[&tag_name])
            .expect("Hard-coded query will return Some([0 or more rows])")
            .get(0)
            .map(|row| row.get(0))
            .ok_or_else(|| warp::reject::custom("Error: Hashtag does not exist."))
    }

    /// Query Postgres for everyone the user has blocked or muted
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub fn select_blocked_users(self, user_id: Id) -> HashSet<Id> {
        let mut conn = self.conn.get().expect("TODO");
        conn.query(
            "SELECT target_account_id FROM blocks WHERE account_id = $1
                 UNION SELECT target_account_id FROM mutes WHERE account_id = $1",
            &[&*user_id],
        )
        .expect("Hard-coded query will return Some([0 or more rows])")
        .iter()
        .map(|row| Id(row.get(0)))
        .collect()
    }
    /// Query Postgres for everyone who has blocked the user
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub fn select_blocking_users(self, user_id: Id) -> HashSet<Id> {
        let mut conn = self.conn.get().expect("TODO");
        conn.query(
            "SELECT account_id FROM blocks WHERE target_account_id = $1",
            &[&*user_id],
        )
        .expect("Hard-coded query will return Some([0 or more rows])")
        .iter()
        .map(|row| Id(row.get(0)))
        .collect()
    }

    /// Query Postgres for all current domain blocks
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub fn select_blocked_domains(self, user_id: Id) -> HashSet<String> {
        let mut conn = self.conn.get().expect("TODO");
        conn.query(
            "SELECT domain FROM account_domain_blocks WHERE account_id = $1",
            &[&*user_id],
        )
        .expect("Hard-coded query will return Some([0 or more rows])")
        .iter()
        .map(|row| row.get(0))
        .collect()
    }

    /// Test whether a user owns a list
    pub fn user_owns_list(self, user_id: Id, list_id: i64) -> bool {
        let mut conn = self.conn.get().expect("TODO");
        // For the Postgres query, `id` = list number; `account_id` = user.id
        let rows = &conn
            .query(
                "SELECT id, account_id FROM lists WHERE id = $1 LIMIT 1",
                &[&list_id],
            )
            .expect("Hard-coded query will return Some([0 or more rows])");
        rows.get(0).map_or(false, |row| Id(row.get(1)) == user_id)
    }
}
