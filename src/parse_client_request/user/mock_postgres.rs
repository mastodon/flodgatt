//! Mock Postgres connection (for use in unit testing)

#[derive(Clone)]
pub struct PostgresPool;
impl PostgresPool {
    pub fn new() -> Self {
        Self
    }
}

pub fn query_for_user_data(
    access_token: &str,
    _pg_pool: PostgresPool,
) -> (i64, String, Option<Vec<String>>, Vec<String>) {
    let (user_id, email, lang, scopes) = if access_token == "TEST_USER" {
        (
            1,
            "user@example.com".to_string(),
            None,
            vec![
                "read".to_string(),
                "write".to_string(),
                "follow".to_string(),
            ],
        )
    } else {
        (-1, "".to_string(), None, Vec::new())
    };
    (user_id, email, lang, scopes)
}

pub fn query_list_owner(list_id: i64, _pg_pool: PostgresPool) -> Option<i64> {
    match list_id {
        1 => Some(1),
        _ => None,
    }
}
