pub mod connection;
mod manager;
pub mod msg;

pub use connection::{RedisConn, RedisConnErr};
pub use manager::{Manager, ManagerErr};
pub use msg::RedisParseErr;

pub enum RedisCmd {
    Subscribe,
    Unsubscribe,
}

impl RedisCmd {
    pub fn into_sendable(&self, tl: &String) -> (Vec<u8>, Vec<u8>) {
        match self {
            RedisCmd::Subscribe => (
                format!("*2\r\n$9\r\nsubscribe\r\n${}\r\n{}\r\n", tl.len(), tl).into_bytes(),
                format!("*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n$1\r\n1\r\n", tl.len(), tl).into_bytes(),
            ),
            RedisCmd::Unsubscribe => (
                format!("*2\r\n$11\r\nunsubscribe\r\n${}\r\n{}\r\n", tl.len(), tl).into_bytes(),
                format!("*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n$1\r\n0\r\n", tl.len(), tl).into_bytes(),
            ),
        }
    }
}
