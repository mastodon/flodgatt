use super::query;
use postgres;
use warp::Filter;

pub fn get_token() -> warp::filters::BoxedFilter<(String,)> {
    let token_from_header = warp::header::header::<String>("authorization")
        .map(|auth: String| auth.split(' ').nth(1).unwrap_or("invalid").to_string());

    let token_from_query = warp::query().map(|q: query::Auth| q.access_token);
    token_from_query.or(token_from_header).unify().boxed()
}

pub fn get_account_id_from_token(token: String) -> Result<i64, warp::reject::Rejection> {
    if let Ok(account_id) = get_account_id(token) {
        Ok(account_id)
    } else {
        Err(warp::reject::custom("Error: Invalid access token"))
    }
}

fn conn() -> postgres::Connection {
    postgres::Connection::connect(
        "postgres://dsock@localhost/mastodon_development",
        postgres::TlsMode::None,
    )
    .unwrap()
}

pub fn get_account_id(token: String) -> Result<i64, ()> {
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
        let account_id: i64 = only_row.get(1);
        Ok(account_id)
    } else {
        Err(())
    }
}
