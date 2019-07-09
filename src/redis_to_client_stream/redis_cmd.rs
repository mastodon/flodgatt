//! Send raw TCP commands to the Redis server
use log::info;
use std::fmt::Display;

/// Send a subscribe or unsubscribe to the Redis PubSub channel
#[macro_export]
macro_rules! pubsub_cmd {
    ($cmd:expr, $self:expr, $tl:expr) => {{
        info!("Sending {} command to {}", $cmd, $tl);
        $self
            .pubsub_connection
            .write_all(&redis_cmd::pubsub($cmd, $tl))
            .expect("Can send command to Redis");
        let new_value = if $cmd == "subscribe" { "1" } else { "0" };
        $self
            .secondary_redis_connection
            .write_all(&redis_cmd::set(
                format!("subscribed:timeline:{}", $tl),
                new_value,
            ))
            .expect("Can set Redis");
        info!("Now subscribed to: {:#?}", $self.msg_queues);
    }};
}
/// Send a `SUBSCRIBE` or `UNSUBSCRIBE` command to a specific timeline
pub fn pubsub(command: impl Display, timeline: impl Display) -> Vec<u8> {
    let arg = format!("timeline:{}", timeline);
    let command = command.to_string();
    info!("Sent {} command", &command);
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
