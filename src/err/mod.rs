mod timeline;

pub use timeline::TimelineErr;

use crate::redis_to_client_stream::ReceiverErr;
use std::fmt;

pub enum FatalErr {
    Err,
    ReceiverErr(ReceiverErr),
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
        write!(f, "Error message")
    }
}

impl From<ReceiverErr> for FatalErr {
    fn from(e: ReceiverErr) -> Self {
        Self::ReceiverErr(e)
    }
}
pub fn die_with_msg2(msg: impl fmt::Display) {
    eprintln!("{}", msg);
    std::process::exit(1);
}

pub fn die_with_msg(msg: impl fmt::Display) -> ! {
    eprintln!("FATAL ERROR: {}", msg);
    std::process::exit(1);
}

#[macro_export]
macro_rules! log_fatal {
    ($str:expr, $var:expr) => {{
        log::error!($str, $var);
        panic!();
    };};
}
