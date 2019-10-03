//! Mock Postgres connection (for use in unit testing)
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct PostgresConn(Arc<Mutex<String>>);
impl PostgresConn {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new("MOCK".to_string())))
    }
}
pub fn query_for_user_data(
    access_token: &str,
    _pg_conn: PostgresConn,
) -> (i64, Option<Vec<String>>, Vec<String>) {
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

pub fn query_list_owner(list_id: i64, _pg_conn: PostgresConn) -> Option<i64> {
    match list_id {
        1 => Some(1),
        _ => None,
    }
}
