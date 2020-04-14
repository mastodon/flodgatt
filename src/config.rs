pub use {deployment_cfg::Deployment, postgres_cfg::Postgres, redis_cfg::Redis};

use self::environmental_variables::EnvVar;
use super::err::FatalErr;
use hashbrown::HashMap;
use std::env;

mod deployment_cfg;
mod deployment_cfg_types;
mod environmental_variables;
mod postgres_cfg;
mod postgres_cfg_types;
mod redis_cfg;
mod redis_cfg_types;

type Result<T> = std::result::Result<T, FatalErr>;

pub fn merge_dotenv() -> Result<()> {
    let env_file = match env::var("ENV").ok().as_deref() {
        Some("production") => ".env.production",
        Some("development") | None => ".env",
        Some(v) => Err(FatalErr::config("ENV", v, "`production` or `development`"))?,
    };
    let res = dotenv::from_filename(env_file);

    if let Ok(log_level) = env::var("RUST_LOG") {
        if res.is_err() && ["warn", "info", "trace", "debug"].contains(&log_level.as_str()) {
            eprintln!(
                 " WARN: could not load environmental variables from {:?}\n\
                  {:8}Are you in the right directory?  Proceeding with variables from the environment.",
                env::current_dir().unwrap_or_else(|_|"./".into()).join(env_file), ""

            );
        }
    }
    Ok(())
}

#[allow(clippy::implicit_hasher)]
pub fn from_env<'a>(
    env_vars: HashMap<String, String>,
) -> Result<(Postgres, Redis, Deployment<'a>)> {
    let env_vars = EnvVar::new(env_vars);
    log::info!(
        "Flodgatt received the following environmental variables:{}",
        &env_vars
    );

    let pg_cfg = Postgres::from_env(env_vars.clone())?;
    log::info!("Configuration for {:#?}", &pg_cfg);
    let redis_cfg = Redis::from_env(env_vars.clone())?;
    log::info!("Configuration for {:#?},", &redis_cfg);
    let deployment_cfg = Deployment::from_env(&env_vars)?;
    log::info!("Configuration for {:#?}", &deployment_cfg);

    Ok((pg_cfg, redis_cfg, deployment_cfg))
}
