//! `User` struct and related functionality
#[cfg(test)]
mod mock_postgres;
#[cfg(test)]
use mock_postgres as postgres;
#[cfg(not(test))]
mod postgres;
pub use self::postgres::PostgresPool as PgPool;
use super::query::Query;
use warp::reject::Rejection;

/// The filters that can be applied to toots after they come from Redis
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    NoFilter,
    Language,
    Notification,
}
impl Default for Filter {
    fn default() -> Self {
        Filter::NoFilter
    }
}

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
    domain_blocks: Vec<String>,
    user_blocks: Vec<i64>,
}

/// The User (with data read from Postgres)
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub target_timeline: String,
    pub email: String, // We only use email for logging; we could cut it for performance
    pub access_token: String, // We only need this once (to send back with the WS reply).  Cut?
    pub id: i64,
    pub scopes: OauthScope,
    pub langs: Option<Vec<String>>,
    pub logged_in: bool,
    pub filter: Filter,
    pub blocks: Blocks,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: -1,
            email: "".to_string(),
            access_token: "".to_string(),
            scopes: OauthScope::default(),
            langs: None,
            logged_in: false,
            target_timeline: String::new(),
            filter: Filter::default(),
            blocks: Blocks::default(),
        }
    }
}

impl User {
    pub fn from_query(q: Query, pool: PgPool) -> Result<Self, Rejection> {
        let mut user: User = match q.access_token.clone() {
            None => User::default(),
            Some(token) => postgres::select_user(&token, pool.clone())?,
        };

        user = user.set_timeline_and_filter(q, pool.clone())?;
        user.blocks.user_blocks = postgres::select_user_blocks(user.id, pool.clone());
        user.blocks.domain_blocks = postgres::select_domain_blocks(pool.clone());

        Ok(user)
    }

    fn set_timeline_and_filter(mut self, q: Query, pool: PgPool) -> Result<Self, Rejection> {
        let read_scope = self.scopes.clone();
        let timeline = match q.stream.as_ref() {
            // Public endpoints:
            tl @ "public" | tl @ "public:local" if q.media => format!("{}:media", tl),
            tl @ "public:media" | tl @ "public:local:media" => tl.to_string(),
            tl @ "public" | tl @ "public:local" => tl.to_string(),
            // Hashtag endpoints:
            tl @ "hashtag" | tl @ "hashtag:local" => format!("{}:{}", tl, q.hashtag),
            // Private endpoints: User:
            "user" if self.logged_in && (read_scope.all || read_scope.statuses) => {
                self.filter = Filter::NoFilter;
                format!("{}", self.id)
            }
            "user:notification" if self.logged_in && (read_scope.all || read_scope.notify) => {
                self.filter = Filter::Notification;
                format!("{}", self.id)
            }
            // List endpoint:
            "list" if self.owns_list(q.list, pool) && (read_scope.all || read_scope.lists) => {
                self.filter = Filter::NoFilter;
                format!("list:{}", q.list)
            }
            // Direct endpoint:
            "direct" if self.logged_in && (read_scope.all || read_scope.statuses) => {
                self.filter = Filter::NoFilter;
                "direct".to_string()
            }
            // Reject unathorized access attempts for private endpoints
            "user" | "user:notification" | "direct" | "list" => {
                return Err(warp::reject::custom("Error: Missing access token"))
            }
            // Other endpoints don't exist:
            _ => return Err(warp::reject::custom("Error: Nonexistent endpoint")),
        };
        Ok(Self {
            target_timeline: timeline,
            ..self
        })
    }

    fn owns_list(&self, list: i64, pool: PgPool) -> bool {
        postgres::user_owns_list(self.id, list, pool)
    }
}
