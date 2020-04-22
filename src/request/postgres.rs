//! Postgres queries
use super::err;
use super::timeline::{Scope, UserData};
use crate::config;
use crate::Id;

use ::postgres::{self, SimpleQueryMessage};
use hashbrown::HashSet;
use r2d2_postgres::PostgresConnectionManager;
use std::convert::TryFrom;
#[allow(deprecated)] // one fn is deprecated, not whole module
use warp::reject;

#[derive(Clone)]
pub struct PgPool {
    conn: r2d2::Pool<PostgresConnectionManager<postgres::NoTls>>,
    whitelist_mode: bool,
}

type Result<T> = std::result::Result<T, err::Error>;
type Rejectable<T> = std::result::Result<T, warp::Rejection>;

impl PgPool {
    pub(crate) const BAD_TOKEN: &'static str = "Error: Missing access token";
    pub(crate) const SERVER_ERR: &'static str = "Error: Internal server error";
    pub(crate) const PG_NULL: &'static str = "Error: Unexpected null from Postgres";
    pub(crate) const MISSING_HASHTAG: &'static str = "Error: Hashtag does not exist";

    pub(crate) fn new(pg_cfg: &config::Postgres, whitelist_mode: bool) -> Result<Self> {
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

    fn is_safe(txt: &str) -> bool {
        txt.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    pub(crate) fn select_user(self, token: &Option<String>) -> Rejectable<UserData> {
        let mut conn = self.conn.get().map_err(reject::custom)?;

        if let Some(token) = token {
            if !Self::is_safe(token) {
                Err(reject::custom(Self::BAD_TOKEN))?;
            };

            let query_rows = conn
                .simple_query(&format!("
SELECT oauth_access_tokens.resource_owner_id, users.account_id, users.chosen_languages, oauth_access_tokens.scopes
  FROM oauth_access_tokens
INNER JOIN users ON oauth_access_tokens.resource_owner_id = users.id
  WHERE oauth_access_tokens.token='{}' AND oauth_access_tokens.revoked_at IS NULL
LIMIT 1", &token.to_owned())
                ).map_err(reject::custom)?;

            let result_columns = match query_rows
                .get(0)
                .ok_or_else(|| reject::custom(Self::SERVER_ERR))?
            {
                postgres::SimpleQueryMessage::Row(row) => row,
                _ => Err(reject::custom(Self::PG_NULL))?, // Wildcard required by #[non_exhaustive]
            };
            let id = Id(get_col_or_reject(result_columns, 1)?
                .parse()
                .map_err(reject::custom)?);

            let allowed_langs = result_columns
                .try_get(2)
                .unwrap_or_default()
                .into_iter()
                .map(String::from)
                .collect();

            let mut scopes: HashSet<Scope> = get_col_or_reject(result_columns, 3)?
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
        } else if self.whitelist_mode {
            Err(reject::custom(Self::BAD_TOKEN))
        } else {
            Ok(UserData::public())
        }
    }

    pub(crate) fn select_hashtag_id(self, tag_name: &str) -> Rejectable<i64> {
        if !Self::is_safe(tag_name) {
            Err(reject::custom(Self::MISSING_HASHTAG))?;
        };

        let mut conn = self.conn.get().map_err(reject::custom)?;
        let rows = conn
            .simple_query(&format!(
                "SELECT id FROM tags WHERE name='{}' LIMIT 1",
                &tag_name
            ))
            .map_err(reject::custom)?;
        match rows.get(0).ok_or_else(|| reject::custom(Self::PG_NULL))? {
            SimpleQueryMessage::Row(row) => get_col_or_reject(row, 0),
            _ => Err(reject::custom(Self::MISSING_HASHTAG))?,
        }
        .map(|s| s.parse().map_err(reject::custom))?
    }

    /// Query Postgres for everyone the user has blocked or muted
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub(crate) fn select_blocked_users(self, user_id: Id) -> Rejectable<HashSet<Id>> {
        let mut conn = self.conn.get().map_err(reject::custom)?;
        conn.simple_query(&format!(
            "SELECT target_account_id FROM blocks WHERE account_id = {0}
                 UNION SELECT target_account_id FROM mutes WHERE account_id = {0}",
            &*user_id
        ))
        .map_err(reject::custom)?
        .iter()
        .map(|msg| match msg {
            postgres::SimpleQueryMessage::Row(row) => Ok(Id(get_col_or_reject(row, 0)?
                .parse()
                .map_err(reject::custom)?)),
            _ => Ok(Id(0)),
        })
        .collect()
    }

    /// Query Postgres for everyone who has blocked the user
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub(crate) fn select_blocking_users(self, user_id: Id) -> Rejectable<HashSet<Id>> {
        let mut conn = self.conn.get().map_err(reject::custom)?;
        conn.simple_query(&format!(
            "SELECT account_id FROM blocks WHERE target_account_id = {}",
            &*user_id
        ))
        .map_err(reject::custom)?
        .iter()
        .map(|msg| match msg {
            postgres::SimpleQueryMessage::Row(row) => Ok(Id(get_col_or_reject(row, 0)?
                .parse()
                .map_err(reject::custom)?)),
            _ => Ok(Id(0)),
        })
        .collect()
    }

    /// Query Postgres for all current domain blocks
    ///
    /// **NOTE**: because we check this when the user connects, it will not include any blocks
    /// the user adds until they refresh/reconnect.
    pub(crate) fn select_blocked_domains(self, user_id: Id) -> Rejectable<HashSet<String>> {
        let mut conn = self.conn.get().map_err(reject::custom)?;
        conn.simple_query(&format!(
            "SELECT domain FROM account_domain_blocks WHERE account_id = {}",
            &*user_id,
        ))
        .map_err(reject::custom)?
        .iter()
        .map(|msg| match msg {
            postgres::SimpleQueryMessage::Row(row) => Ok(get_col_or_reject(row, 0)?.to_string()),
            _ => Ok(String::new()),
        })
        .collect()
    }

    /// Test whether a user owns a list
    pub(crate) fn user_owns_list(self, user_id: Id, list_id: i64) -> Rejectable<bool> {
        // For the Postgres query, `id` = list number; `account_id` = user.id
        let mut conn = self.conn.get().map_err(reject::custom)?;
        let rows = conn
            .simple_query(&format!(
                "SELECT id, account_id FROM lists WHERE id={} LIMIT 1",
                &list_id,
            ))
            .map_err(reject::custom)?;

        match rows.get(0).ok_or_else(|| reject::custom(Self::PG_NULL))? {
            SimpleQueryMessage::Row(row) => {
                Ok(Id(get_col_or_reject(row, 1)?.parse().map_err(reject::custom)?) == user_id)
            }
            _ => Err(reject::custom(Self::MISSING_HASHTAG))?,
        }
    }
}

fn get_col_or_reject(row: &postgres::row::SimpleQueryRow, col: usize) -> Rejectable<&str> {
    Ok(row
        .try_get(col)
        .map_err(reject::custom)?
        .ok_or(reject::custom(PgPool::PG_NULL))?)
}
