mod deployment_cfg;
mod deployment_cfg_types;
mod postgres_cfg;
mod redis_cfg;
pub use self::{
    deployment_cfg::DeploymentConfig, postgres_cfg::PostgresConfig, redis_cfg::RedisConfig,
};

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
macro_rules! derive_deref {
    ($name:ident: $type:ty) => {
        impl std::ops::Deref for $name {
            type Target = $type;
            fn deref(&self) -> &$type {
                &self.0
            }
        }
    };
}
#[macro_export]
macro_rules! derive_oneline_debug {
    ($item:ident) => {
        impl std::fmt::Debug for $item {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.0)
            }
        }
    };
}
#[macro_export]
macro_rules! derive_from_str_or_die {
    ($item:ident {name: $name:expr,
                  value: $value:expr}) => {
        impl FromStrOrDie<Self> for $item {
            fn name_and_values() -> (&'static str, String) {
                ($name, $value)
            }
        }
    };
}

#[macro_export]
macro_rules! from_env_var {
    ($name:ident {
        inner: $inner:expr; $type:ty,
        env_var: $env_var:tt,
        allowed_values: $allowed_values:expr,
    }
     inner_from_str(|$arg:ident| $body:expr)
    ) => {
        pub struct $name {
            pub inner: $type,
            pub env_var: String,
            pub allowed_values: String,
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", self.inner)
            }
        }
        impl std::ops::Deref for $name {
            type Target = $type;
            fn deref(&self) -> &$type {
                &self.inner
            }
        }
        impl $name {
            fn inner_from_str($arg: &str) -> Option<$type> {
                $body
            }
            fn update_inner(&mut self, inner: $type) -> &Self {
                self.inner = inner;
                self
            }
            pub fn default() -> Self {
                $name {
                    inner: $inner,
                    env_var: $env_var.to_string(),
                    allowed_values: $allowed_values,
                }
            }
            pub fn from_env_var_or_die(env: Option<&String>) -> Self {
                let mut res = Self::default();
                if let Some(value) = env {
                    res.update_inner(Self::inner_from_str(value).unwrap_or_else(|| {
                        eprintln!(
                            "\"{}\" is not a valid value for {}.  {} must be {}",
                            value, res.env_var, res.env_var, res.allowed_values
                        );
                        std::process::exit(1);
                    }));
                    res
                } else {
                    res
                }
            }
        }
    };
}
