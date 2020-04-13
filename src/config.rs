pub use {deployment_cfg::Deployment, postgres_cfg::Postgres, redis_cfg::Redis};

use self::environmental_variables::EnvVar;
use super::err;
use hashbrown::HashMap;
use std::env;

mod deployment_cfg;
mod deployment_cfg_types;
mod environmental_variables;
mod postgres_cfg;
mod postgres_cfg_types;
mod redis_cfg;
mod redis_cfg_types;

pub fn merge_dotenv() -> Result<(), err::FatalErr> {
    // TODO -- should this allow the user to run in a dir without a `.env` file?
    dotenv::from_filename(match env::var("ENV").ok().as_deref() {
        Some("production") => ".env.production",
        Some("development") | None => ".env",
        Some(_unsupported) => Err(err::FatalErr::Unknown)?, // TODO make more specific
    })?;
    Ok(())
}

pub fn from_env<'a>(env_vars: HashMap<String, String>) -> (Postgres, Redis, Deployment<'a>) {
    let env_vars = EnvVar::new(env_vars);
    log::info!("Environmental variables Flodgatt received: {}", &env_vars);
    (
        Postgres::from_env(env_vars.clone()),
        Redis::from_env(env_vars.clone()),
        Deployment::from_env(env_vars.clone()),
    )
}
