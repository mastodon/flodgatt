use super::receiver::Receiver;
use futures::{Async, Poll};
use serde_json::Value;
use std::io::Read;
use std::net;
use tokio::io::AsyncRead;

pub struct AsyncReadableStream<'a>(&'a mut net::TcpStream);
impl<'a> AsyncReadableStream<'a> {
    pub fn new(stream: &'a mut net::TcpStream) -> Self {
        AsyncReadableStream(stream)
    }
    /// Polls Redis for any new messages and adds them to the `MsgQueue` for
    /// the appropriate `ClientAgent`.
    pub fn poll_redis(receiver: &mut Receiver) {
        let mut buffer = vec![0u8; 3000];

        let mut async_stream = AsyncReadableStream::new(&mut receiver.pubsub_connection);
        if let Async::Ready(num_bytes_read) = async_stream.poll_read(&mut buffer).unwrap() {
            let raw_redis_response = &String::from_utf8_lossy(&buffer[..num_bytes_read]);
            receiver.incoming_raw_msg.push_str(raw_redis_response);
            // Text comes in from redis as a raw stream, which could be more than one message
            // and is not guaranteed to end on a message boundary.  We need to break it down
            // into messages.  Incoming messages *are* guaranteed to be RESP arrays,
            // https://redis.io/topics/protocol

            // Only act if we have a full message (end on a msg boundary)
            if !receiver.incoming_raw_msg.ends_with("}\r\n") {
                return;
            };
            while receiver.incoming_raw_msg.len() > 0 {
                let mut msg = RedisMsg::from_raw(receiver.incoming_raw_msg.clone());
                let command = msg.get_next_item();
                match command.as_str() {
                    "message" => {
                        let timeline = msg.get_next_item()["timeline:".len()..].to_string();
                        let message: Value = serde_json::from_str(&msg.get_next_item()).unwrap();
                        for msg_queue in receiver.msg_queues.values_mut() {
                            if msg_queue.redis_channel == timeline {
                                msg_queue.messages.push_back(message.clone());
                            }
                        }
                    }
                    "subscribe" | "unsubscribe" => {
                        // This returns a confirmation.  We don't need to do anything with it,
                        // but we do need to advance the cursor past it
                        msg.get_next_item(); // name of channel (un)subscribed
                        msg.cursor += ":".len();
                        msg.process_number(); // The number of active subscriptions
                        msg.cursor += "\r\n".len();
                    }
                    cmd => panic!(
                        "Invariant violation: bad Redis input.  Got {} as a command",
                        cmd
                    ),
                }
                receiver.incoming_raw_msg = msg.raw[msg.cursor..].to_string();
            }
        }
    }
}

impl<'a> Read for AsyncReadableStream<'a> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.0.read(buffer)
    }
}

impl<'a> AsyncRead for AsyncReadableStream<'a> {
    fn poll_read(&mut self, buf: &mut [u8]) -> Poll<usize, std::io::Error> {
        match self.read(buf) {
            Ok(t) => Ok(Async::Ready(t)),
            Err(_) => Ok(Async::NotReady),
        }
    }
}

#[derive(Default)]
pub struct RedisMsg {
    pub raw: String,
    pub cursor: usize,
}
impl RedisMsg {
    pub fn from_raw(raw: String) -> Self {
        Self {
            raw,
            cursor: "*3\r\n".len(), //length of intro header
            ..Self::default()
        }
    }
    /// Move the cursor from the beginning of a number through its end and return the number
    pub fn process_number(&mut self) -> usize {
        let mut selection_end = self.cursor + 1;
        let mut chars = self.raw.chars();
        chars.nth(self.cursor);
        while chars.next().expect("still in str").is_digit(10) {
            selection_end += 1;
        }
        let selected_number = self.raw[self.cursor..selection_end]
            .parse::<usize>()
            .expect("checked with `.is_digit(10)`");
        self.cursor = selection_end;
        selected_number
    }
    /// In a pubsub reply from Redis, an item can be either the name of the subscribed channel
    /// or the msg payload.  Either way, it follows the same format:
    /// `$[LENGTH_OF_ITEM_BODY]\r\n[ITEM_BODY]\r\n`
    pub fn get_next_item(&mut self) -> String {
        self.cursor += "$".len();
        let item_len = self.process_number();
        self.cursor += "\r\n".len();
        let item_start_position = self.cursor;
        self.cursor += item_len;
        let item = self.raw[item_start_position..self.cursor].to_string();
        self.cursor += "\r\n".len();
        item
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_redis_parse() {
        let input = "*3\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntimeline:1\r\n:1\r\n";
        let mut msg = RedisMsg::from_raw(input.to_string());
        let cmd = msg.get_next_item();
        assert_eq!(&cmd, "SUBSCRIBE");
        let timeline = msg.get_next_item();
        assert_eq!(&timeline, "timeline:1");
        msg.cursor += ":1\r\n".len();
        assert_eq!(msg.cursor, input.len());
    }

    #[test]
    fn realistic_redis_parse() {
        let input = "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:4\r\n$1386\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102866835379605039\",\"created_at\":\"2019-09-27T22:29:02.590Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"http://localhost:3000/users/admin/statuses/102866835379605039\",\"url\":\"http://localhost:3000/@admin/102866835379605039\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"http://localhost:3000/@susan\\\" class=\\\"u-url mention\\\">@<span>susan</span></a></span> hi</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"1\",\"username\":\"admin\",\"acct\":\"admin\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-07-04T00:21:05.890Z\",\"note\":\"<p></p>\",\"url\":\"http://localhost:3000/@admin\",\"avatar\":\"http://localhost:3000/avatars/original/missing.png\",\"avatar_static\":\"http://localhost:3000/avatars/original/missing.png\",\"header\":\"http://localhost:3000/headers/original/missing.png\",\"header_static\":\"http://localhost:3000/headers/original/missing.png\",\"followers_count\":3,\"following_count\":3,\"statuses_count\":192,\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"4\",\"username\":\"susan\",\"url\":\"http://localhost:3000/@susan\",\"acct\":\"susan\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1569623342825}\r\n";
        let mut msg = RedisMsg::from_raw(input.to_string());
        let cmd = msg.get_next_item();
        assert_eq!(&cmd, "message");
        let timeline = msg.get_next_item();
        assert_eq!(&timeline, "timeline:4");
        let message_str = msg.get_next_item();
        assert_eq!(message_str, input[41..input.len() - 2]);
        assert_eq!(msg.cursor, input.len());
    }
}
