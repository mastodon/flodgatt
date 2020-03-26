use std::fmt::Display;

pub fn die_with_msg(msg: impl Display) -> ! {
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
