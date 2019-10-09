mod deployment_cfg;
mod deployment_cfg_types;
mod postgres_cfg;
mod redis_cfg;
mod redis_cfg_types;
pub use self::{
    deployment_cfg::DeploymentConfig,
    postgres_cfg::PostgresConfig,
    redis_cfg::RedisConfig,
    redis_cfg_types::{RedisInterval, RedisNamespace},
};
use std::collections::HashMap;
use url::Url;

pub struct EnvVar(pub HashMap<String, String>);
impl std::ops::Deref for EnvVar {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &HashMap<String, String> {
        &self.0
    }
}
impl Clone for EnvVar {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl EnvVar {
    fn update_with_url(mut self, url_str: &str) -> Self {
        let url = Url::parse(url_str).unwrap();
        let none_if_empty = |s: String| if s.is_empty() { None } else { Some(s) };

        self.maybe_add_env_var("REDIS_PORT", url.port());
        self.maybe_add_env_var("REDIS_PASSWORD", url.password());
        self.maybe_add_env_var("REDIS_USERNAME", none_if_empty(url.username().to_string()));
        self.maybe_add_env_var("REDIS_DB", none_if_empty(url.path()[1..].to_string()));
        for (k, v) in url.query_pairs().into_owned() {
            match k.to_string().as_str() {
                "password" => self.maybe_add_env_var("REDIS_PASSWORD", Some(v.to_string())),
                "db" => self.maybe_add_env_var("REDIS_DB", Some(v.to_string())),
                _ => crate::err::die_with_msg(format!(
                    r"Unsupported parameter {} in REDIS_URL.
             Flodgatt supports only `password` and `db` parameters.",
                    k
                )),
            }
        }

        self
    }
    fn maybe_add_env_var(&mut self, key: &str, maybe_value: Option<impl ToString>) {
        if let Some(value) = maybe_value {
            self.0.insert(key.to_string(), value.to_string());
        }
    }
}

#[macro_export]
macro_rules! maybe_update {
    ($name:ident; $item: tt:$type:ty) => (
        pub fn $name(self, item: Option<$type>) -> Self {
            match item {
                Some($item) => Self{ $item, ..self },
                None => Self { ..self }
            }
        });
    ($name:ident; Some($item: tt: $type:ty)) => (
        fn $name(self, item: Option<$type>) -> Self{
            match item {
                Some($item) => Self{ $item: Some($item), ..self },
                None => Self { ..self }
            }
        })}
#[macro_export]
macro_rules! from_env_var {
    ($(#[$outer:meta])*
     let name = $name:ident;
     let default: $type:ty = $inner:expr;
     let (env_var, allowed_values) = ($env_var:tt, $allowed_values:expr);
     let from_str = |$arg:ident| $body:expr;
    ) => {
        pub struct $name(pub $type);
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.0)
            }
        }
        impl std::ops::Deref for $name {
            type Target = $type;
            fn deref(&self) -> &$type {
                &self.0
            }
        }
        impl std::default::Default for $name {
            fn default() -> Self {
                $name($inner)
            }
        }
        impl $name {
            fn inner_from_str($arg: &str) -> Option<$type> {
                $body
            }
            pub fn maybe_update(self, var: Option<&String>) -> Self {
                if let Some(value) = var {
                    Self(Self::inner_from_str(value).unwrap_or_else(|| {
                        crate::err::env_var_fatal($env_var, value, $allowed_values)
                    }))
                } else {
                    self
                }
            }
        }
    };
}
