mod timeline;

pub use timeline::TimelineErr;

use std::fmt;

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
