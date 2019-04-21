use crate::{or, query};
use postgres;
use warp::Filter;

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

pub struct User {
    pub id: String,
    pub langs: Vec<String>,
    pub logged_in: bool,
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
            id: id.to_string(),
            langs,
            logged_in: true,
        })
    } else if let Scope::Public = scope {
        Ok(User {
            id: String::new(),
            langs: Vec::new(),
            logged_in: false,
        })
    } else {
        Err(warp::reject::custom("Error: Invalid access token"))
    }
}
