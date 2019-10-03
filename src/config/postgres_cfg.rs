use url::Url;

#[derive(Debug)]
pub struct PostgresConfig {
    pub user: String,
    pub host: String,
    pub password: Option<String>,
    pub database: String,
    pub port: String,
    pub ssl_mode: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            user: "postgres".to_string(),
            host: "localhost".to_string(),
            password: None,
            database: "mastodon_development".to_string(),
            port: "5432".to_string(),
            ssl_mode: "prefer".to_string(),
        }
    }
}
fn none_if_empty(item: &str) -> Option<String> {
    if item.is_empty() {
        None
    } else {
        Some(item.to_string())
    }
}
macro_rules! maybe_update {
    ( $name:ident; $item: tt ) => (
        pub fn $name(self, item: Option<String>) -> Self{
            match item {
                Some($item) => Self{ $item, ..self },
                _ => Self { ..self }
            }
        })}

impl PostgresConfig {
    maybe_update!(maybe_update_user; user);
    maybe_update!(maybe_update_host; host);
    maybe_update!(maybe_update_db; database);
    maybe_update!(maybe_update_port; port);
    maybe_update!(maybe_update_sslmode; ssl_mode);
    pub fn maybe_update_pass(self, pass: Option<String>) -> Self {
        match pass {
            Some(password) => Self {
                password: Some(password),
                ..self
            },
            _ => Self { ..self },
        }
    }

    pub fn from_url(url: Url) -> Self {
        let ssl_mode = url
            .query_pairs()
            .find(|(key, _val)| key.to_string().as_str() == "sslmode")
            .map(|(_key, val)| val.to_string());

        Self::default()
            .maybe_update_user(none_if_empty(url.username()))
            .maybe_update_host(url.host_str().map(String::from))
            .maybe_update_pass(url.password().map(String::from))
            .maybe_update_port(url.port().map(|port_num| port_num.to_string()))
            .maybe_update_db(none_if_empty(url.path()))
            .maybe_update_sslmode(ssl_mode)
    }
}
