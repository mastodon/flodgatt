use super::{postgres_cfg_types::*, EnvVar};
use url::Url;

#[derive(Debug)]
pub struct PostgresConfig {
    pub user: PgUser,
    pub host: PgHost,
    pub password: PgPass,
    pub database: PgDatabase,
    pub port: PgPort,
    pub ssl_mode: PgSslMode,
}

impl EnvVar {
    fn update_with_postgres_url(mut self, url_str: &str) -> Self {
        let url = Url::parse(url_str).unwrap();
        let none_if_empty = |s: String| if s.is_empty() { None } else { Some(s) };

        for (k, v) in url.query_pairs().into_owned() {
            match k.to_string().as_str() {
                "user" => self.maybe_add_env_var("DB_USER", Some(v.to_string())),
                "password" => self.maybe_add_env_var("DB_PASS", Some(v.to_string())),
                "host" => self.maybe_add_env_var("DB_HOST", Some(v.to_string())),
                "sslmode" => self.maybe_add_env_var("DB_SSLMODE", Some(v.to_string())),
                _ => crate::err::die_with_msg(format!(
                    r"Unsupported parameter {} in POSTGRES_URL
             Flodgatt supports only `password`, `user`, `host`, and `sslmode` parameters",
                    k
                )),
            }
        }

        self.maybe_add_env_var("DB_PORT", url.port());
        self.maybe_add_env_var("DB_PASS", url.password());
        self.maybe_add_env_var("DB_USER", none_if_empty(url.username().to_string()));
        self.maybe_add_env_var("DB_NAME", none_if_empty(url.path()[1..].to_string()));

        self
    }
}

impl PostgresConfig {
    /// Configure Postgres and return a connection

    pub fn from_env(env: EnvVar) -> Self {
        let env = match env.get("DATABASE_URL").cloned() {
            Some(url_str) => env.update_with_postgres_url(&url_str),
            None => env,
        };

        let cfg = Self {
            user: PgUser::default().maybe_update(env.get("DB_USER")),
            host: PgHost::default().maybe_update(env.get("DB_HOST")),
            password: PgPass::default().maybe_update(env.get("DB_PASS")),
            database: PgDatabase::default().maybe_update(env.get("DB_NAME")),
            port: PgPort::default().maybe_update(env.get("DB_PORT")),
            ssl_mode: PgSslMode::default().maybe_update(env.get("DB_SSLMODE")),
        };

        log::warn!("Postgres configuration:\n{:#?}", &cfg);

        cfg
    }

    //     // use openssl::ssl::{SslConnector, SslMethod};
    //     // use postgres_openssl::MakeTlsConnector;
    //     // let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
    //     // builder.set_ca_file("/etc/ssl/cert.pem").unwrap();
    //     // let connector = MakeTlsConnector::new(builder.build());
    //     // TODO: add TLS support, remove `NoTls`
}
