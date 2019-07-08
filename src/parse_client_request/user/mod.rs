//! `User` struct and related functionality
mod postgres;
use crate::parse_client_request::query;
use log::info;
use warp::Filter as WarpFilter;

/// Combine multiple routes with the same return type together with
/// `or()` and `unify()`
#[macro_export]
macro_rules! any_of {
    ($filter:expr, $($other_filter:expr),*) => {
        $filter$(.or($other_filter).unify())*
    };
}

/// The filters that can be applied to toots after they come from Redis
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    NoFilter,
    Language,
    Notification,
}

/// The User (with data read from Postgres)
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: i64,
    pub access_token: String,
    pub scopes: OauthScope,
    pub langs: Option<Vec<String>>,
    pub logged_in: bool,
    pub filter: Filter,
}
impl Default for User {
    fn default() -> Self {
        User::public()
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

/// Create a user based on the supplied path and access scope for the resource
#[macro_export]
macro_rules! user_from_path {
    ($($path_item:tt) / *, $scope:expr) => (path!("api" / "v1" / $($path_item) / +)
                                              .and($scope.get_access_token())
                                              .and_then(|token| User::from_access_token(token, $scope)))
}

impl User {
    /// Create a user from the access token supplied in the header or query paramaters
    pub fn from_access_token(
        access_token: String,
        scope: Scope,
    ) -> Result<Self, warp::reject::Rejection> {
        let (id, langs, scope_list) = postgres::query_for_user_data(&access_token);
        let scopes = OauthScope::from(scope_list);
        if id != -1 || scope == Scope::Public {
            let (logged_in, log_msg) = match id {
                -1 => (false, "Public access to non-authenticated endpoints"),
                _ => (true, "Granting logged-in access"),
            };
            info!("{}", log_msg);
            Ok(User {
                id,
                access_token,
                scopes,
                langs,
                logged_in,
                filter: Filter::NoFilter,
            })
        } else {
            Err(warp::reject::custom("Error: Invalid access token"))
        }
    }
    /// Set the Notification/Language filter
    pub fn set_filter(self, filter: Filter) -> Self {
        Self { filter, ..self }
    }
    /// Determine whether the User is authorised for a specified list
    pub fn owns_list(&self, list: i64) -> bool {
        match postgres::query_list_owner(list) {
            Some(i) if i == self.id => true,
            _ => false,
        }
    }
    /// A public (non-authenticated) User
    pub fn public() -> Self {
        User {
            id: -1,
            access_token: String::from("no access token"),
            scopes: OauthScope::default(),
            langs: None,
            logged_in: false,
            filter: Filter::NoFilter,
        }
    }
}

/// Whether the endpoint requires authentication or not
#[derive(PartialEq)]
pub enum Scope {
    Public,
    Private,
}
impl Scope {
    pub fn get_access_token(self) -> warp::filters::BoxedFilter<(String,)> {
        let token_from_header_http_push = warp::header::header::<String>("authorization")
            .map(|auth: String| auth.split(' ').nth(1).unwrap_or("invalid").to_string());
        let token_from_header_ws =
            warp::header::header::<String>("Sec-WebSocket-Protocol").map(|auth: String| auth);
        let token_from_query = warp::query().map(|q: query::Auth| q.access_token);

        let private_scopes = any_of!(
            token_from_header_http_push,
            token_from_header_ws,
            token_from_query
        );

        let public = warp::any().map(|| "no access token".to_string());

        match self {
            // if they're trying to access a private scope without an access token, reject the request
            Scope::Private => private_scopes.boxed(),
            // if they're trying to access a public scope without an access token, proceed
            Scope::Public => any_of!(private_scopes, public).boxed(),
        }
    }
}
