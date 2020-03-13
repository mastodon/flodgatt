//! `User` struct and related functionality
#[cfg(test)]
mod mock_postgres;
#[cfg(test)]
use mock_postgres as postgres;
#[cfg(not(test))]
mod postgres;
pub use self::postgres::PgPool;
use super::query::Query;
use std::collections::HashSet;
use warp::reject::Rejection;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OauthScope {
    pub all: bool,
    pub statuses: bool,
    pub notify: bool,
    pub lists: bool,
}
impl From<Vec<String>> for OauthScope {
    fn from(scope_list: Vec<String>) -> Self {
        let mut oauth_scope = OauthScope::default();
        for scope in scope_list {
            match scope.as_str() {
                "read" => oauth_scope.all = true,
                "read:statuses" => oauth_scope.statuses = true,
                "read:notifications" => oauth_scope.notify = true,
                "read:lists" => oauth_scope.lists = true,
                _ => (),
            }
        }
        oauth_scope
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Blocks {
    pub domain_blocks: HashSet<String>,
    pub user_blocks: HashSet<i64>,
}

/// The User (with data read from Postgres)
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub target_timeline: String,
    pub email: String, // We only use email for logging; we could cut it for performance
    pub access_token: String, // We only need this once (to send back with the WS reply).  Cut?
    pub id: i64,
    pub scopes: OauthScope,
    pub logged_in: bool,
    pub allowed_langs: HashSet<String>,
    pub blocks: Blocks,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: -1,
            email: "".to_string(),
            access_token: "".to_string(),
            scopes: OauthScope::default(),
            logged_in: false,
            target_timeline: String::new(),
            allowed_langs: HashSet::new(),
            blocks: Blocks::default(),
        }
    }
}

impl User {
    pub fn from_query(q: Query, pool: PgPool) -> Result<Self, Rejection> {
        println!("Creating user...");
        let mut user: User = match q.access_token.clone() {
            None => User::default(),
            Some(token) => postgres::select_user(&token, pool.clone())?,
        };

        user = user.set_timeline_and_filter(q, pool.clone())?;
        user.blocks.user_blocks = postgres::select_user_blocks(user.id, pool.clone());
        user.blocks.domain_blocks = postgres::select_domain_blocks(pool.clone());
        dbg!(&user);
        Ok(user)
    }

    fn set_timeline_and_filter(self, q: Query, pool: PgPool) -> Result<Self, Rejection> {
        let (read_scope, f) = (self.scopes.clone(), self.allowed_langs.clone());
        let (filter, target_timeline) = match q.stream.as_ref() {
            // Public endpoints:
            tl @ "public" | tl @ "public:local" if q.media => (f, format!("{}:media", tl)),
            tl @ "public:media" | tl @ "public:local:media" => (f, tl.to_string()),
            tl @ "public" | tl @ "public:local" => (f, tl.to_string()),

            // Hashtag endpoints:
            tl @ "hashtag" | tl @ "hashtag:local" => (f, format!("{}:{}", tl, q.hashtag)),
            // Private endpoints: User:
            "user" if self.logged_in && (read_scope.all || read_scope.statuses) => {
                (HashSet::new(), format!("{}", self.id))
            }
            "user:notification" if self.logged_in && (read_scope.all || read_scope.notify) => {
                (HashSet::new(), format!("{}", self.id))
            }
            // List endpoint:
            "list" if self.owns_list(q.list, pool) && (read_scope.all || read_scope.lists) => {
                (HashSet::new(), format!("list:{}", q.list))
            }
            // Direct endpoint:
            "direct" if self.logged_in && (read_scope.all || read_scope.statuses) => {
                (HashSet::new(), "direct".to_string())
            }
            // Reject unathorized access attempts for private endpoints
            "user" | "user:notification" | "direct" | "list" => {
                return Err(warp::reject::custom("Error: Missing access token"))
            }
            // Other endpoints don't exist:
            _ => return Err(warp::reject::custom("Error: Nonexistent endpoint")),
        };
        Ok(Self {
            target_timeline,
            allowed_langs: filter,
            ..self
        })
    }

    fn owns_list(&self, list: i64, pool: PgPool) -> bool {
        postgres::user_owns_list(self.id, list, pool)
    }
}
