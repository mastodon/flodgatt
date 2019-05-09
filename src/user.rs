//! Create a User by querying the Postgres database with the user's access_token
use crate::{any_of, query};
use log::info;
use postgres;
use warp::Filter as WarpFilter;

/// (currently hardcoded to localhost)
pub fn connect_to_postgres() -> postgres::Connection {
    postgres::Connection::connect(
        "postgres://dsock@localhost/mastodon_development",
        postgres::TlsMode::None,
    )
    .expect("Can connect to local Postgres")
}

/// The filters that can be applied to toots after they come from Redis
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    None,
    Language,
    Notification,
}

/// The User (with data read from Postgres)
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: i64,
    pub langs: Option<Vec<String>>,
    pub logged_in: bool,
    pub filter: Filter,
}
impl User {
    /// Create a user from the access token supplied in the header or query paramaters
    pub fn from_access_token(token: String, scope: Scope) -> Result<Self, warp::reject::Rejection> {
        let conn = connect_to_postgres();
        let result = &conn
            .query(
                "
SELECT oauth_access_tokens.resource_owner_id, users.account_id, users.chosen_languages
FROM
oauth_access_tokens
INNER JOIN users ON
oauth_access_tokens.resource_owner_id = users.id
WHERE oauth_access_tokens.token = $1
AND oauth_access_tokens.revoked_at IS NULL
LIMIT 1",
                &[&token],
            )
            .expect("Hard-coded query will return Some([0 or more rows])");
        if !result.is_empty() {
            let only_row = result.get(0);
            let id: i64 = only_row.get(1);
            let langs: Option<Vec<String>> = only_row.get(2);
            info!("Granting logged-in access");
            Ok(User {
                id,
                langs,
                logged_in: true,
                filter: Filter::None,
            })
        } else if let Scope::Public = scope {
            println!("Granting public access");
            Ok(User {
                id: -1,
                langs: None,
                logged_in: false,
                filter: Filter::None,
            })
        } else {
            Err(warp::reject::custom("Error: Invalid access token"))
        }
    }
    /// Add a Notification filter
    pub fn with_notification_filter(self) -> Self {
        Self {
            filter: Filter::Notification,
            ..self
        }
    }
    /// Add a Language filter
    pub fn with_language_filter(self) -> Self {
        Self {
            filter: Filter::Language,
            ..self
        }
    }
    /// Remove all filters
    pub fn with_no_filter(self) -> Self {
        Self {
            filter: Filter::None,
            ..self
        }
    }
    /// Determine whether the User is authorised for a specified list
    pub fn authorized_for_list(&self, list: i64) -> Result<i64, warp::reject::Rejection> {
        let conn = connect_to_postgres();
        // For the Postgres query, `id` = list number; `account_id` = user.id
        let rows = &conn
            .query(
                " SELECT id, account_id FROM lists WHERE id = $1 LIMIT 1",
                &[&list],
            )
            .expect("Hard-coded query will return Some([0 or more rows])");
        if !rows.is_empty() {
            let id_of_account_that_owns_the_list: i64 = rows.get(0).get(1);
            if id_of_account_that_owns_the_list == self.id {
                return Ok(list);
            }
        };

        Err(warp::reject::custom("Error: Invalid access token"))
    }
    /// A public (non-authenticated) User
    pub fn public() -> Self {
        User {
            id: -1,
            langs: None,
            logged_in: false,
            filter: Filter::None,
        }
    }
}

/// Whether the endpoint requires authentication or not
pub enum Scope {
    Public,
    Private,
}
impl Scope {
    pub fn get_access_token(self) -> warp::filters::BoxedFilter<(String,)> {
        let token_from_header = warp::header::header::<String>("authorization")
            .map(|auth: String| auth.split(' ').nth(1).unwrap_or("invalid").to_string());
        let token_from_query = warp::query().map(|q: query::Auth| q.access_token);
        let public = warp::any().map(|| "no access token".to_string());

        match self {
            // if they're trying to access a private scope without an access token, reject the request
            Scope::Private => any_of!(token_from_query, token_from_header).boxed(),
            // if they're trying to access a public scope without an access token, proceed
            Scope::Public => any_of!(token_from_query, token_from_header, public).boxed(),
        }
    }
}
