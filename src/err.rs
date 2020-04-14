use crate::request::RequestErr;
use crate::response::ManagerErr;
use std::fmt;

pub enum FatalErr {
    Unknown,
    ReceiverErr(ManagerErr),
    DotEnv(dotenv::Error),
    Logger(log::SetLoggerError),
    Postgres(RequestErr),
}

impl FatalErr {
    pub fn exit(msg: impl fmt::Display) {
        eprintln!("{}", msg);
        std::process::exit(1);
    }
}

impl std::error::Error for FatalErr {}
impl fmt::Debug for FatalErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl fmt::Display for FatalErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use FatalErr::*;
        write!(
            f,
            "{}",
            match self {
                Unknown => "Flodgatt encountered an unknown, unrecoverable error".into(),
                ReceiverErr(e) => format!("{}", e),
                Logger(e) => format!("{}", e),
                DotEnv(e) => format!("Could not load specified environmental file: {}", e),
                Postgres(e) => format!("Could not connect to Postgres: {}", e),
            }
        )
    }
}

impl From<RequestErr> for FatalErr {
    fn from(e: RequestErr) -> Self {
        Self::Postgres(e)
    }
}

impl From<dotenv::Error> for FatalErr {
    fn from(e: dotenv::Error) -> Self {
        Self::DotEnv(e)
    }
}

impl From<ManagerErr> for FatalErr {
    fn from(e: ManagerErr) -> Self {
        Self::ReceiverErr(e)
    }
}

impl From<log::SetLoggerError> for FatalErr {
    fn from(e: log::SetLoggerError) -> Self {
        Self::Logger(e)
    }
}

// TODO delete vvvv when postgres_cfg.rs has better error handling
pub fn die_with_msg(msg: impl fmt::Display) -> ! {
    eprintln!("FATAL ERROR: {}", msg);
    std::process::exit(1);
}
