use super::receiver::Receiver;
use crate::config;
use futures::{Async, Poll};
use serde_json::Value;
use std::io::Read;
use std::net;
use tokio::io::AsyncRead;

pub struct AsyncReadableStream<'a>(&'a mut net::TcpStream);

impl<'a> AsyncReadableStream<'a> {
    pub fn new(stream: &'a mut net::TcpStream) -> Self {
        Self(stream)
    }

    // Text comes in from redis as a raw stream, which could be more than one message
    // and is not guaranteed to end on a message boundary.  We need to break it down
    // into messages.  Incoming messages *are* guaranteed to be RESP arrays,
    // https://redis.io/topics/protocol
    /// Adds any new Redis messages to the `MsgQueue` for the appropriate `ClientAgent`.
    pub fn poll_redis(receiver: &mut Receiver) {
        let mut buffer = vec![0u8; 6000];
        let mut async_stream = AsyncReadableStream::new(&mut receiver.pubsub_connection);

        if let Async::Ready(num_bytes_read) = async_stream.poll_read(&mut buffer).unwrap() {
            let raw_redis_response = async_stream.to_utf8(buffer, num_bytes_read);
            if raw_redis_response.starts_with("-NOAUTH") {
                eprintln!(
                    r"Invalid authentication for Redis.
Do you need a password?
If so, set it with the REDIS_PASSWORD environmental variable"
                );
                std::process::exit(1);
            }

            receiver.incoming_raw_msg.push_str(&raw_redis_response);

            // Only act if we have a full message (end on a msg boundary)
            if !receiver.incoming_raw_msg.ends_with("}\r\n") {
                return;
            };
            let mut msg = RedisMsg::from_raw(&receiver.incoming_raw_msg);

            let prefix_to_skip = match &*config::REDIS_NAMESPACE {
                Some(namespace) => format!("{}:timeline:", namespace),
                None => "timeline:".to_string(),
            };

            while !msg.raw.is_empty() {
                let command = msg.next_field();
                match command.as_str() {
                    "message" => {
                        let timeline = &msg.next_field()[prefix_to_skip.len()..];
                        let msg_txt = &msg.next_field();
                        let msg_value: Value = match serde_json::from_str(msg_txt) {
                            Ok(v) => v,
                            Err(e) => panic!("Unparseable json {}\n\n{}", msg_txt, e),
                        };
                        dbg!(&timeline);
                        for msg_queue in receiver.msg_queues.values_mut() {
                            if msg_queue.redis_channel == timeline {
                                msg_queue.messages.push_back(msg_value.clone());
                            }
                        }
                    }
                    "subscribe" | "unsubscribe" => {
                        // No msg, so ignore & advance cursor to end
                        let _channel = msg.next_field();
                        msg.cursor += ":".len();
                        let _active_subscriptions = msg.process_number();
                        msg.cursor += "\r\n".len();
                    }
                    cmd => panic!("Invariant violation: {} is invalid Redis input", cmd),
                };
                msg = RedisMsg::from_raw(&msg.raw[msg.cursor..]);
            }
            receiver.incoming_raw_msg.clear();
        }
    }

    fn to_utf8(&mut self, cur_buffer: Vec<u8>, size: usize) -> String {
        String::from_utf8(cur_buffer[..size].to_vec()).unwrap_or_else(|_| {
            let mut new_buffer = vec![0u8; 1];
            self.poll_read(&mut new_buffer).unwrap();
            let buffer = ([cur_buffer, new_buffer]).concat();
            self.to_utf8(buffer, size + 1)
        })
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

#[derive(Debug)]
pub struct RedisMsg<'a> {
    pub raw: &'a str,
    pub cursor: usize,
}

impl<'a> RedisMsg<'a> {
    pub fn from_raw(raw: &'a str) -> Self {
        Self {
            raw,
            cursor: "*3\r\n".len(), //length of intro header
        }
    }
    /// Move the cursor from the beginning of a number through its end and return the number
    pub fn process_number(&mut self) -> usize {
        let (mut selected_number, selection_start) = (0, self.cursor);
        while let Ok(number) = self.raw[selection_start..=self.cursor].parse::<usize>() {
            self.cursor += 1;
            selected_number = number;
        }
        selected_number
    }
    /// In a pubsub reply from Redis, an item can be either the name of the subscribed channel
    /// or the msg payload.  Either way, it follows the same format:
    /// `$[LENGTH_OF_ITEM_BODY]\r\n[ITEM_BODY]\r\n`
    pub fn next_field(&mut self) -> String {
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
        let mut msg = RedisMsg::from_raw(input);
        let cmd = msg.next_field();
        assert_eq!(&cmd, "SUBSCRIBE");
        let timeline = msg.next_field();
        assert_eq!(&timeline, "timeline:1");
        msg.cursor += ":1\r\n".len();
        assert_eq!(msg.cursor, input.len());
    }

    #[test]
    fn realistic_redis_parse() {
        let input = "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:4\r\n$1386\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102866835379605039\",\"created_at\":\"2019-09-27T22:29:02.590Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"public\",\"language\":\"en\",\"uri\":\"http://localhost:3000/users/admin/statuses/102866835379605039\",\"url\":\"http://localhost:3000/@admin/102866835379605039\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p><span class=\\\"h-card\\\"><a href=\\\"http://localhost:3000/@susan\\\" class=\\\"u-url mention\\\">@<span>susan</span></a></span> hi</p>\",\"reblog\":null,\"application\":{\"name\":\"Web\",\"website\":null},\"account\":{\"id\":\"1\",\"username\":\"admin\",\"acct\":\"admin\",\"display_name\":\"\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-07-04T00:21:05.890Z\",\"note\":\"<p></p>\",\"url\":\"http://localhost:3000/@admin\",\"avatar\":\"http://localhost:3000/avatars/original/missing.png\",\"avatar_static\":\"http://localhost:3000/avatars/original/missing.png\",\"header\":\"http://localhost:3000/headers/original/missing.png\",\"header_static\":\"http://localhost:3000/headers/original/missing.png\",\"followers_count\":3,\"following_count\":3,\"statuses_count\":192,\"emojis\":[],\"fields\":[]},\"media_attachments\":[],\"mentions\":[{\"id\":\"4\",\"username\":\"susan\",\"url\":\"http://localhost:3000/@susan\",\"acct\":\"susan\"}],\"tags\":[],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1569623342825}\r\n";
        let mut msg = RedisMsg::from_raw(input);
        let cmd = msg.next_field();
        assert_eq!(&cmd, "message");
        let timeline = msg.next_field();
        assert_eq!(&timeline, "timeline:4");
        let message_str = msg.next_field();
        assert_eq!(message_str, input[41..input.len() - 2]);
        assert_eq!(msg.cursor, input.len());
    }
}
