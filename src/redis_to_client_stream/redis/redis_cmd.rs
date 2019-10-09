//! Send raw TCP commands to the Redis server
use std::fmt::Display;

/// Send a subscribe or unsubscribe to the Redis PubSub channel
#[macro_export]
macro_rules! pubsub_cmd {
    ($cmd:expr, $self:expr, $tl:expr) => {{
        use std::io::Write;
        log::info!("Sending {} command to {}", $cmd, $tl);
        let namespace = $self.pubsub_connection.namespace.clone();

        $self
            .pubsub_connection
            .write_all(&redis_cmd::pubsub($cmd, $tl, namespace.clone()))
            .expect("Can send command to Redis");
        // Because we keep track of the number of clients subscribed to a channel on our end,
        // we need to manually tell Redis when we have subscribed or unsubscribed
        let subscription_new_number = match $cmd {
            "unsubscribe" => "0",
            "subscribe" => "1",
            _ => panic!("Given unacceptable PUBSUB command"),
        };
        $self
            .secondary_redis_connection
            .write_all(&redis_cmd::set(
                format!("subscribed:timeline:{}", $tl),
                subscription_new_number,
                namespace.clone(),
            ))
            .expect("Can set Redis");

        log::info!("Now subscribed to: {:#?}", $self.msg_queues);
    }};
}
/// Send a `SUBSCRIBE` or `UNSUBSCRIBE` command to a specific timeline
pub fn pubsub(command: impl Display, timeline: impl Display, ns: Option<String>) -> Vec<u8> {
    let arg = match ns {
        Some(namespace) => format!("{}:timeline:{}", namespace, timeline),
        None => format!("timeline:{}", timeline),
    };
    cmd(command, arg)
}

/// Send a generic two-item command to Redis
pub fn cmd(command: impl Display, arg: impl Display) -> Vec<u8> {
    let (command, arg) = (command.to_string(), arg.to_string());
    log::info!("Sent {} command", &command);
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

/// Send a `SET` command (used to manually unsubscribe from Redis)
pub fn set(key: impl Display, value: impl Display, ns: Option<String>) -> Vec<u8> {
    let key = match ns {
        Some(namespace) => format!("{}:{}", namespace, key),
        None => key.to_string(),
    };
    let value = value.to_string();
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
