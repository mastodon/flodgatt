//! Postgres queries
use crate::config;

#[cfg(not(test))]
pub fn query_for_user_data(access_token: &str) -> (i64, Option<Vec<String>>, Vec<String>) {
    let mut conn = config::postgres();

    let query_result = conn
            .query(
                "
SELECT oauth_access_tokens.resource_owner_id, users.account_id, users.chosen_languages, oauth_access_tokens.scopes
FROM
oauth_access_tokens
INNER JOIN users ON
oauth_access_tokens.resource_owner_id = users.id
WHERE oauth_access_tokens.token = $1
AND oauth_access_tokens.revoked_at IS NULL
LIMIT 1",
                &[&access_token.to_owned()],
            )
            .expect("Hard-coded query will return Some([0 or more rows])");
    if !query_result.is_empty() {
        let only_row: &postgres::Row = query_result.get(0).unwrap();
        let id: i64 = only_row.get(1);
        let scopes = only_row
            .get::<_, String>(3)
            .split(' ')
            .map(|s| s.to_owned())
            .collect();
        let langs: Option<Vec<String>> = only_row.get(2);
        (id, langs, scopes)
    } else {
        (-1, None, Vec::new())
    }
}

#[cfg(test)]
pub fn query_for_user_data(access_token: &str) -> (i64, Option<Vec<String>>, Vec<String>) {
    let (user_id, lang, scopes) = if access_token == "TEST_USER" {
        (
            1,
            None,
            vec![
                "read".to_string(),
                "write".to_string(),
                "follow".to_string(),
            ],
        )
    } else {
        (-1, None, Vec::new())
    };
    (user_id, lang, scopes)
}

#[cfg(not(test))]
pub fn query_list_owner(list_id: i64) -> Option<i64> {
    let mut conn = config::postgres();
    // For the Postgres query, `id` = list number; `account_id` = user.id
    let rows = &conn
        .query(
            "
SELECT id, account_id
FROM lists
WHERE id = $1
LIMIT 1",
            &[&list_id],
        )
        .expect("Hard-coded query will return Some([0 or more rows])");
    if rows.is_empty() {
        None
    } else {
        Some(rows.get(0).unwrap().get(1))
    }
}

#[cfg(test)]
pub fn query_list_owner(_list_id: i64) -> Option<i64> {
    Some(1)
}
