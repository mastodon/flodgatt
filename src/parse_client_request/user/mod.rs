//! `User` struct and related functionality
//#[cfg(test)]
//mod mock_postgres;
//#[cfg(test)]
//use mock_postgres as postgres;
//#[cfg(not(test))]
mod postgres;
pub use self::postgres::PostgresPool;
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

/// The User (with data read from Postgres)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct User {
    pub target_timeline: String,
    pub id: i64,
    pub access_token: String,
    pub scopes: OauthScope,
    pub langs: Option<Vec<String>>,
    pub logged_in: bool,
    pub filter: Filter,
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

impl User {
    pub fn from_query(q: Query, pg_conn: PostgresPool) -> Result<Self, Rejection> {
        let (id, access_token, scopes, langs, logged_in) = match q.access_token.clone() {
            None => (
                -1,
                "no access token".to_owned(),
                OauthScope::default(),
                None,
                false,
            ),
            Some(token) => {
                let (id, langs, scope_list) =
                    postgres::query_for_user_data(&token, pg_conn.clone());
                if id == -1 {
                    return Err(warp::reject::custom("Error: Invalid access token"));
                }
                let scopes = OauthScope::from(scope_list);
                (id, token, scopes, langs, true)
            }
        };
        let mut user = User {
            id,
            target_timeline: "PLACEHOLDER".to_string(),
            access_token,
            scopes,
            langs,
            logged_in,
            filter: Filter::Language,
        };

        user = user.update_timeline_and_filter(q, pg_conn.clone())?;

        Ok(user)
    }

    fn update_timeline_and_filter(
        mut self,
        q: Query,
        pg_conn: PostgresPool,
    ) -> Result<Self, Rejection> {
        let read_scope = self.scopes.clone();

        let timeline = match q.stream.as_ref() {
            // Public endpoints:
            tl @ "public" | tl @ "public:local" if q.media => format!("{}:media", tl),
            tl @ "public:media" | tl @ "public:local:media" => tl.to_string(),
            tl @ "public" | tl @ "public:local" => tl.to_string(),
            // Hashtag endpoints:
            tl @ "hashtag" | tl @ "hashtag:local" => format!("{}:{}", tl, q.hashtag),
            // Private endpoints: User
            "user" if self.logged_in && (read_scope.all || read_scope.statuses) => {
                self.filter = Filter::NoFilter;
                format!("{}", self.id)
            }
            "user:notification" if self.logged_in && (read_scope.all || read_scope.notify) => {
                self.filter = Filter::Notification;
                format!("{}", self.id)
            }
            // List endpoint:
            "list" if self.owns_list(q.list, pg_conn) && (read_scope.all || read_scope.lists) => {
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

    /// Determine whether the User is authorised for a specified list
    pub fn owns_list(&self, list: i64, pg_conn: PostgresPool) -> bool {
        match postgres::query_list_owner(list, pg_conn) {
            Some(i) if i == self.id => true,
            _ => false,
        }
    }
}
