use crate::from_env_var;

from_env_var!(
    /// The host address where Redis is running
    let name = PgHost;
    let default: IpAddr = IpAddr::V4("127.0.0.1".parse().expect("hardcoded"));
    let (env_var, allowed_values) = ("", "a valid address (e.g., 127.0.0.1)".to_string());
    let from_str = |s| match s {
        "localhost" => Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        _ => s.parse().ok(),
    };
);
