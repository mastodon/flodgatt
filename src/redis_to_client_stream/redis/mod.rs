pub mod redis_cmd;
pub mod redis_connection;
pub mod redis_msg;
pub mod redis_stream;

pub use redis_connection::RedisConn;
pub use redis_stream::RedisStream;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_redis_parse() {
        let input = "*3\r\n$9\r\nSUBSCRIBE\r\n$10\r\ntimeline:1\r\n:1\r\n";
        let mut msg = redis_msg::RedisMsg::from_raw(input, "timeline".len());
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
        let mut msg = redis_msg::RedisMsg::from_raw(input, "timeline".len());
        let cmd = msg.next_field();
        assert_eq!(&cmd, "message");
        let timeline = msg.next_field();
        assert_eq!(&timeline, "timeline:4");
        let message_str = msg.next_field();
        assert_eq!(message_str, input[41..input.len() - 2]);
        assert_eq!(msg.cursor, input.len());
    }
}
