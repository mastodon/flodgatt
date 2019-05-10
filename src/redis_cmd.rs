//! Send raw TCP commands to the Redis server
use std::fmt::Display;

/// Send a `SUBSCRIBE` or `UNSUBSCRIBE` command to a specific timeline
pub fn pubsub(command: impl Display, timeline: impl Display) -> Vec<u8> {
    let arg = format!("timeline:{}", timeline);
    let command = command.to_string();
    format!(
        "*2\r\n${cmd_length}\r\n{cmd}\r\n${arg_length}\r\n{arg}\r\n",
        cmd_length = command.len(),
        cmd = command,
        arg_length = arg.len(),
        arg = arg
    )
    .as_bytes()
    .to_owned()
}

/// Send a `SET` command
pub fn set(key: impl Display, value: impl Display) -> Vec<u8> {
    let (key, value) = (key.to_string(), value.to_string());
    format!(
        "*3\r\n$3\r\nSET\r\n${key_length}\r\n{key}\r\n${value_length}\r\n{value}\r\n",
        key_length = key.len(),
        key = key,
        value_length = value.len(),
        value = value
    )
    .as_bytes()
    .to_owned()
}
