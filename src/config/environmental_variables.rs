use hashbrown::HashMap;
use std::fmt;

#[derive(Debug)]
pub(crate) struct EnvVar(pub HashMap<String, String>);
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
    pub(crate) fn new(vars: HashMap<String, String>) -> Self {
        Self(vars)
    }

    pub(crate) fn maybe_add_env_var(&mut self, key: &str, maybe_value: Option<impl ToString>) {
        if let Some(value) = maybe_value {
            self.0.insert(key.to_string(), value.to_string());
        }
    }
}
impl fmt::Display for EnvVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = String::new();
        for env_var in &[
            "NODE_ENV",
            "RUST_LOG",
            "BIND",
            "PORT",
            "SOCKET",
            "SSE_FREQ",
            "WS_FREQ",
            "DATABASE_URL",
            "DB_USER",
            "USER",
            "DB_PORT",
            "DB_HOST",
            "DB_PASS",
            "DB_NAME",
            "DB_SSLMODE",
            "REDIS_HOST",
            "REDIS_USER",
            "REDIS_PORT",
            "REDIS_PASSWORD",
            "REDIS_USER",
            "REDIS_DB",
            "REDIS_FREQ",
        ] {
            if let Some(value) = self.get(&(*env_var).to_string()) {
                result = format!("{}\n    {}: {}", result, env_var, value)
            }
        }
        write!(f, "{}", result)
    }
}
#[macro_export]
macro_rules! maybe_update {
    ($name:ident; $item: tt:$type:ty) => (
        pub(crate) fn $name(self, item: Option<$type>) -> Self {
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
        #[derive(Clone)]
        pub struct $name(pub $type);
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
            pub(crate) fn maybe_update(
                self,
                var: Option<&String>,
            ) -> Result<Self, crate::err::FatalErr> {
                Ok(match var {
                    Some(empty_string) if empty_string.is_empty() => Self::default(),
                    Some(value) => Self(Self::inner_from_str(value).ok_or_else(|| {
                        crate::err::FatalErr::config($env_var, value, $allowed_values)
                    })?),
                    None => self,
                })
            }
        }
    };
}
