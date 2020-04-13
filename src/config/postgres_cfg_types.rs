use crate::from_env_var;
use std::str::FromStr;
use strum_macros::{EnumString, EnumVariantNames};

from_env_var!(
    /// The user to use for Postgres
    let name = PgUser;
    let default: String = "postgres".to_string();
    let (env_var, allowed_values) = ("DB_USER", "any string");
    let from_str = |s| Some(s.to_string());
);

from_env_var!(
    /// The host address where Postgres is running)
    let name = PgHost;
    let default: String = "localhost".to_string();
    let (env_var, allowed_values) = ("DB_HOST", "any string");
    let from_str = |s| Some(s.to_string());
);

from_env_var!(
    /// The password to use with Postgress
    let name = PgPass;
    let default: Option<String> = None;
    let (env_var, allowed_values) = ("DB_PASS", "any string");
    let from_str = |s| Some(Some(s.to_string()));
);

from_env_var!(
    /// The Postgres database to use
    let name = PgDatabase;
    let default: String = "mastodon_development".to_string();
    let (env_var, allowed_values) = ("DB_NAME", "any string");
    let from_str = |s| Some(s.to_string());
);

from_env_var!(
    /// The port Postgres is running on
    let name = PgPort;
    let default: u16 = 5432;
    let (env_var, allowed_values) = ("DB_PORT", "a number between 0 and 65535");
    let from_str = |s| s.parse().ok();
);

from_env_var!(
    let name = PgSslMode;
    let default: PgSslInner = PgSslInner::Prefer;
    let (env_var, allowed_values) = ("DB_SSLMODE", &format!("one of: {:?}", PgSslInner::variants()));
    let from_str = |s| PgSslInner::from_str(s).ok();
);

#[derive(EnumString, EnumVariantNames, Debug, Clone)]
#[strum(serialize_all = "snake_case")]
pub enum PgSslInner {
    Prefer,
}
