use crate::{or, query};
use postgres;
use warp::Filter as WarpFilter;

pub fn get_access_token(scope: Scope) -> warp::filters::BoxedFilter<(String,)> {
    let token_from_header = warp::header::header::<String>("authorization")
        .map(|auth: String| auth.split(' ').nth(1).unwrap_or("invalid").to_string());
    let token_from_query = warp::query().map(|q: query::Auth| q.access_token);
    let public = warp::any().map(|| "no access token".to_string());

    match scope {
        // if they're trying to access a private scope without an access token, reject the request
        Scope::Private => or!(token_from_query, token_from_header).boxed(),
        // if they're trying to access a public scope without an access token, proceed
        Scope::Public => or!(token_from_query, token_from_header, public).boxed(),
    }
}

fn conn() -> postgres::Connection {
    postgres::Connection::connect(
        "postgres://dsock@localhost/mastodon_development",
        postgres::TlsMode::None,
    )
    .unwrap()
}
#[derive(Clone)]
pub enum Filter {
    None,
    Language,
    Notification,
}

#[derive(Clone)]
pub struct User {
    pub id: i64,
    pub langs: Vec<String>,
    pub logged_in: bool,
    pub filter: Filter,
}
impl User {
    pub fn with_notification_filter(self) -> Self {
        Self {
            filter: Filter::Notification,
            ..self
        }
    }
    pub fn with_language_filter(self) -> Self {
        Self {
            filter: Filter::Language,
            ..self
        }
    }
    pub fn with_no_filter(self) -> Self {
        Self {
            filter: Filter::None,
            ..self
        }
    }
    pub fn is_authorized_for_list(self, list: i64) -> Result<(i64, User), warp::reject::Rejection> {
        let conn = conn();
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
                return Ok((list, self));
            }
        };

        Err(warp::reject::custom("Error: Invalid access token"))
    }
    pub fn public() -> Self {
        User {
            id: -1,
            langs: Vec::new(),
            logged_in: false,
            filter: Filter::None,
        }
    }
}

pub enum Scope {
    Public,
    Private,
}
pub fn get_account(token: String, scope: Scope) -> Result<User, warp::reject::Rejection> {
    let conn = conn();
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
        let langs: Vec<String> = only_row.get(2);
        Ok(User {
            id: id,
            langs,
            logged_in: true,
            filter: Filter::None,
        })
    } else if let Scope::Public = scope {
        Ok(User {
            id: -1,
            langs: Vec::new(),
            logged_in: false,
            filter: Filter::None,
        })
    } else {
        Err(warp::reject::custom("Error: Invalid access token"))
    }
}
