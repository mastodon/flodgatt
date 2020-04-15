//! Postgres queries
use super::err;
use super::timeline::{Scope, UserData};
use crate::config;
use crate::event::Id;

use ::postgres;
use hashbrown::HashSet;
use r2d2_postgres::PostgresConnectionManager;
use std::convert::TryFrom;

#[derive(Clone, Debug)]
pub struct PgPool {
    pub conn: r2d2::Pool<PostgresConnectionManager<postgres::NoTls>>,
    whitelist_mode: bool,
}

type Result<T> = std::result::Result<T, err::RequestErr>;
type Rejectable<T> = std::result::Result<T, warp::Rejection>;

impl PgPool {
    pub fn new(pg_cfg: &config::Postgres, whitelist_mode: bool) -> Result<Self> {
        let mut cfg = postgres::Config::new();
        cfg.user(&pg_cfg.user)
            .host(&*pg_cfg.host.to_string())
            .port(*pg_cfg.port)
            .dbname(&pg_cfg.database);
        if let Some(password) = &*pg_cfg.password {
            cfg.password(password);
        };

        cfg.connect(postgres::NoTls)?; // Test connection, letting us immediately exit with an error
                                       // when Postgres isn't running instead of timing out below
        let manager = PostgresConnectionManager::new(cfg, postgres::NoTls);
        let pool = r2d2::Pool::builder().max_size(10).build(manager)?;

        Ok(Self {
            conn: pool,
            whitelist_mode,
        })
    }

    pub fn select_user(self, token: &Option<String>) -> Rejectable<UserData> {
        let mut conn = self.conn.get().map_err(warp::reject::custom)?;

        if let Some(token) = token {
            let query_rows = conn
                .query("
SELECT oauth_access_tokens.resource_owner_id, users.account_id, users.chosen_languages, oauth_access_tokens.scopes
  FROM oauth_access_tokens
INNER JOIN users ON oauth_access_tokens.resource_owner_id = users.id
  WHERE oauth_access_tokens.token = $1 AND oauth_access_tokens.revoked_at IS NULL
LIMIT 1",
                       &[&token.to_owned()],
                ).map_err(warp::reject::custom)?;

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

    pub fn select_hashtag_id(self, tag_name: &str) -> Rejectable<i64> {
        let mut conn = self.conn.get().map_err(warp::reject::custom)?;
        conn.query("SELECT id FROM tags WHERE name = $1 LIMIT 1", &[&tag_name])
            .map_err(warp::reject::custom)?
            .get(0)
            .map(|row| row.get(0))
            .ok_or_else(|| warp::reject::custom("Error: Hashtag does not exist."))
    }

    /// Query Postgres for everyone the user has blocked or muted
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub fn select_blocked_users(self, user_id: Id) -> Rejectable<HashSet<Id>> {
        let mut conn = self.conn.get().map_err(warp::reject::custom)?;
        conn.query(
            "SELECT target_account_id FROM blocks WHERE account_id = $1
                 UNION SELECT target_account_id FROM mutes WHERE account_id = $1",
            &[&*user_id],
        )
        .map_err(warp::reject::custom)?
        .iter()
        .map(|row| Ok(Id(row.get(0))))
        .collect()
    }
    /// Query Postgres for everyone who has blocked the user
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub fn select_blocking_users(self, user_id: Id) -> Rejectable<HashSet<Id>> {
        let mut conn = self.conn.get().map_err(warp::reject::custom)?;
        conn.query(
            "SELECT account_id FROM blocks WHERE target_account_id = $1",
            &[&*user_id],
        )
        .map_err(warp::reject::custom)?
        .iter()
        .map(|row| Ok(Id(row.get(0))))
        .collect()
    }

    /// Query Postgres for all current domain blocks
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub fn select_blocked_domains(self, user_id: Id) -> Rejectable<HashSet<String>> {
        let mut conn = self.conn.get().map_err(warp::reject::custom)?;
        conn.query(
            "SELECT domain FROM account_domain_blocks WHERE account_id = $1",
            &[&*user_id],
        )
        .map_err(warp::reject::custom)?
        .iter()
        .map(|row| Ok(row.get(0)))
        .collect()
    }

    /// Test whether a user owns a list
    pub fn user_owns_list(self, user_id: Id, list_id: i64) -> Rejectable<bool> {
        let mut conn = self.conn.get().map_err(warp::reject::custom)?;
        // For the Postgres query, `id` = list number; `account_id` = user.id
        let rows = &conn
            .query(
                "SELECT id, account_id FROM lists WHERE id = $1 LIMIT 1",
                &[&list_id],
            )
            .map_err(warp::reject::custom)?;
        Ok(rows.get(0).map_or(false, |row| Id(row.get(1)) == user_id))
    }
}
