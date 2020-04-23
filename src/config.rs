pub use self::deployment_cfg::Deployment;
pub use self::postgres_cfg::Postgres;
pub use self::redis_cfg::Redis;

use self::environmental_variables::EnvVar;

use hashbrown::HashMap;
use std::env;
use std::fmt;
mod deployment_cfg;
mod deployment_cfg_types;
mod environmental_variables;
mod postgres_cfg;
mod postgres_cfg_types;
mod redis_cfg;
mod redis_cfg_types;

type Result<T> = std::result::Result<T, Error>;

pub fn merge_dotenv() -> Result<()> {
    let env_file = match env::var("ENV").ok().as_deref() {
        Some("production") => ".env.production",
        Some("development") | None => ".env",
        Some(v) => Err(Error::config("ENV", v, "`production` or `development`"))?,
    };
    let res = dotenv::from_filename(env_file);

    if let Ok(log_level) = env::var("RUST_LOG") {
        if ["warn", "info", "trace", "debug"].contains(&log_level.as_str()) {
            let env_file = env::current_dir()
                .unwrap_or_else(|_| "./".into())
                .join(env_file);

            match res {
                Err(dotenv::Error::LineParse(msg, line)) => eprintln!(
                    " ERROR: could not parse environmental file at {:?}\n\
                     {:8}could not parse line {}, `{}`",
                    env_file, "", line, msg
                ),
                Err(dotenv::Error::Io(_)) => eprintln!(
                    " WARN: could not load environmental variables from {:?}\n\
                      {:8}Are you in the right directory?  Proceeding with variables from the environment.",
                    env_file, ""
                ),
                Err(_) => eprintln!(" ERROR: could not load environmental file at {:?}", env_file),
                Ok(_) => ()
            }
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

#[derive(Debug)]
pub enum Error {
    Config(String),
    UrlEncoding(urlencoding::FromUrlEncodingError),
    UrlParse(url::ParseError),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::Config(e) => e.to_string(),
                Self::UrlEncoding(e) => format!("could not parse POSTGRES_URL.\n{:7}{:?}", "", e),
                Self::UrlParse(e) => format!("could parse Postgres URL.\n{:7}{}", "", e),
            }
        )
    }
}

impl Error {
    pub fn config<T: fmt::Display>(var: T, value: T, allowed_vals: T) -> Self {
        Self::Config(format!(
            "{0} is set to `{1}`, which is invalid.\n{3:7}{0} must be {2}.",
            var, value, allowed_vals, ""
        ))
    }
}

impl From<urlencoding::FromUrlEncodingError> for Error {
    fn from(e: urlencoding::FromUrlEncodingError) -> Self {
        Self::UrlEncoding(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::UrlParse(e)
    }
}
