mod connection;
mod manager;
mod msg;

pub(self) use super::{Event, EventErr};
pub(self) use connection::RedisConn;
pub use manager::Error;
pub use manager::Manager;

#[cfg(feature = "bench")]
pub use msg::{RedisMsg, RedisParseOutput};

use connection::RedisConnErr;
use msg::RedisParseErr;

enum RedisCmd {
    Subscribe,
    Unsubscribe,
}

impl RedisCmd {
    fn into_sendable(self, timelines: &[String]) -> (Vec<u8>, Vec<u8>) {
        match self {
            RedisCmd::Subscribe => {
                let primary = {
                    let mut cmd = format!("*{}\r\n$9\r\nsubscribe\r\n", 1 + timelines.len());
                    for tl in timelines {
                        cmd.push_str(&format!("${}\r\n{}\r\n", tl.len(), tl));
                    }
                    cmd
                };
                let secondary = {
                    let mut cmd = format!("*{}\r\n$4\r\nMSET\r\n", 1 + timelines.len());
                    for tl in timelines {
                        cmd.push_str(&format!(
                            "${}\r\nsubscribed:{}\r\n$1\r\n$1\r\n",
                            "subscribed:".len() + tl.len(),
                            tl
                        ));
                    }
                    cmd
                };
                (primary.as_bytes().to_vec(), secondary.as_bytes().to_vec())
            }
            RedisCmd::Unsubscribe => {
                let primary = {
                    let mut cmd = format!("*{}\r\n$11\r\nunsubscribe\r\n", 1 + timelines.len());
                    for tl in timelines {
                        cmd.push_str(&format!("${}\r\n{}\r\n", tl.len(), tl));
                    }
                    cmd
                };
                let secondary = {
                    let mut cmd = format!("*{}\r\n$4\r\nMSET\r\n", 1 + timelines.len());
                    for tl in timelines {
                        cmd.push_str(&format!(
                            "${}\r\nsubscribed:{}\r\n$1\r\n$0\r\n",
                            "subscribed:".len() + tl.len(),
                            tl
                        ));
                    }
                    cmd
                };
                (primary.as_bytes().to_vec(), secondary.as_bytes().to_vec())
            }
        }
    }
}
