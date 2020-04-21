mod connection;
mod manager;
mod msg;

pub(crate) use connection::{RedisConn, RedisConnErr};
pub use manager::Manager;
pub(crate) use manager::ManagerErr;
pub(crate) use msg::RedisParseErr;

pub(crate) enum RedisCmd {
    Subscribe,
    Unsubscribe,
}

impl RedisCmd {
    pub(crate) fn into_sendable(self, tl: &str) -> (Vec<u8>, Vec<u8>) {
        match self {
            RedisCmd::Subscribe => (
                [
                    b"*2\r\n$9\r\nsubscribe\r\n$",
                    tl.len().to_string().as_bytes(),
                    b"\r\n",
                    tl.as_bytes(),
                    b"\r\n",
                ]
                .concat(),
                [
                    b"*3\r\n$3\r\nSET\r\n$",
                    tl.len().to_string().as_bytes(),
                    b"\r\n",
                    tl.as_bytes(),
                    b"\r\n$1\r\n1\r\n",
                ]
                .concat(),
            ),
            RedisCmd::Unsubscribe => (
                [
                    b"*2\r\n$11\r\nunsubscribe\r\n$",
                    tl.len().to_string().as_bytes(),
                    b"\r\n",
                    tl.as_bytes(),
                    b"\r\n",
                ]
                .concat(),
                [
                    b"*3\r\n$3\r\nSET\r\n$",
                    tl.len().to_string().as_bytes(),
                    b"\r\n",
                    tl.as_bytes(),
                    b"\r\n$1\r\n0\r\n",
                ]
                .concat(),
            ),
        }
    }
}
