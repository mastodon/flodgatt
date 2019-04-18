use postgres;

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
