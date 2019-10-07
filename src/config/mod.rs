//! Configuration defaults.  All settings with the prefix of `DEFAULT_` can be overridden
//! by an environmental variable of the same name without that prefix (either by setting
//! the variable at runtime or in the `.env` file)
mod deployment_cfg;
mod deployment_cfg_types;
mod postgres_cfg;
mod redis_cfg;
pub use self::{
    deployment_cfg::DeploymentConfig, postgres_cfg::PostgresConfig, redis_cfg::RedisConfig,
};

// **NOTE**:  Polling Redis is much more time consuming than polling the `Receiver`
//            (on the order of 10ms rather than 50μs).  Thus, changing this setting
//            would be a good place to start for performance improvements at the cost
//            of delaying all updates.

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
