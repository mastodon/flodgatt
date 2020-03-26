use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;

const ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS: &str = "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:1\r\n$3790\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102775370117886890\",\"created_at\":\"2019-09-11T18:42:19.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"unlisted\",\"language\":\"en\",\"uri\":\"https://mastodon.host/users/federationbot/statuses/102775346916917099\",\"url\":\"https://mastodon.host/@federationbot/102775346916917099\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p>Trending tags:<br><a href=\\\"https://mastodon.host/tags/neverforget\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>neverforget</span></a><br><a href=\\\"https://mastodon.host/tags/4styles\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>4styles</span></a><br><a href=\\\"https://mastodon.host/tags/newpipe\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>newpipe</span></a><br><a href=\\\"https://mastodon.host/tags/uber\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>uber</span></a><br><a href=\\\"https://mastodon.host/tags/mercredifiction\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>mercredifiction</span></a></p>\",\"reblog\":null,\"account\":{\"id\":\"78\",\"username\":\"federationbot\",\"acct\":\"federationbot@mastodon.host\",\"display_name\":\"Federation Bot\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-09-10T15:04:25.559Z\",\"note\":\"<p>Hello, I am mastodon.host official semi bot.</p><p>Follow me if you want to have some updates on the view of the fediverse from here ( I only post unlisted ). </p><p>I also randomly boost one of my followers toot every hour !</p><p>If you don\'t feel confortable with me following you, tell me: unfollow  and I\'ll do it :)</p><p>If you want me to follow you, just tell me follow ! </p><p>If you want automatic follow for new users on your instance and you are an instance admin, contact me !</p><p>Other commands are private :)</p>\",\"url\":\"https://mastodon.host/@federationbot\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":16636,\"following_count\":179532,\"statuses_count\":50554,\"emojis\":[],\"fields\":[{\"name\":\"More stats\",\"value\":\"<a href=\\\"https://mastodon.host/stats.html\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/stats.html</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"More infos\",\"value\":\"<a href=\\\"https://mastodon.host/about/more\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/about/more</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Owner/Friend\",\"value\":\"<span class=\\\"h-card\\\"><a href=\\\"https://mastodon.host/@gled\\\" class=\\\"u-url mention\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">@<span>gled</span></a></span>\",\"verified_at\":null}]},\"media_attachments\":[],\"mentions\":[],\"tags\":[{\"name\":\"4styles\",\"url\":\"https://instance.codesections.com/tags/4styles\"},{\"name\":\"neverforget\",\"url\":\"https://instance.codesections.com/tags/neverforget\"},{\"name\":\"mercredifiction\",\"url\":\"https://instance.codesections.com/tags/mercredifiction\"},{\"name\":\"uber\",\"url\":\"https://instance.codesections.com/tags/uber\"},{\"name\":\"newpipe\",\"url\":\"https://instance.codesections.com/tags/newpipe\"}],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1568227693541}\r\n";

/// Parses the Redis message using a Regex.
///
/// The naive approach from Flodgatt's proof-of-concept stage.
mod regex_parse {
    use regex::Regex;
    use serde_json::Value;

    pub fn to_json_value(input: String) -> Value {
        if input.ends_with("}\r\n") {
            let messages = input.as_str().split("message").skip(1);
            let regex = Regex::new(r"timeline:(?P<timeline>.*?)\r\n\$\d+\r\n(?P<value>.*?)\r\n")
                .expect("Hard-codded");
            for message in messages {
                let _timeline = regex.captures(message).expect("Hard-coded timeline regex")
                    ["timeline"]
                    .to_string();

                let redis_msg: Value = serde_json::from_str(
                    &regex.captures(message).expect("Hard-coded value regex")["value"],
                )
                .expect("Valid json");

                return redis_msg;
            }
            unreachable!()
        } else {
            unreachable!()
        }
    }
}

/// Parse with a simplified inline iterator.
///
/// Essentially shows best-case performance for producing a serde_json::Value.
mod parse_inline {
    use serde_json::Value;
    pub fn to_json_value(input: String) -> Value {
        fn print_next_str(mut end: usize, input: &str) -> (usize, String) {
            let mut start = end + 3;
            end = start + 1;

            let mut iter = input.chars();
            iter.nth(start);

            while iter.next().unwrap().is_digit(10) {
                end += 1;
            }
            let length = &input[start..end].parse::<usize>().unwrap();
            start = end + 2;
            end = start + length;

            let string = &input[start..end];
            (end, string.to_string())
        }

        if input.ends_with("}\r\n") {
            let end = 2;
            let (end, _) = print_next_str(end, &input);
            let (end, _timeline) = print_next_str(end, &input);
            let (_, msg) = print_next_str(end, &input);
            let redis_msg: Value = serde_json::from_str(&msg).unwrap();
            redis_msg
        } else {
            unreachable!()
        }
    }
}

/// Parse using Flodgatt's current functions
mod flodgatt_parse_event {
    use flodgatt::{messages::Event, redis_to_client_stream::receiver::MessageQueues};
    use flodgatt::{
        parse_client_request::Timeline,
        redis_to_client_stream::{receiver::MsgQueue, redis::redis_stream},
    };
    use lru::LruCache;
    use std::collections::HashMap;
    use uuid::Uuid;

    /// One-time setup, not included in testing time.
    pub fn setup() -> MessageQueues {
        let mut queues_map = HashMap::new();
        let id = Uuid::default();
        let timeline = Timeline::from_redis_raw_timeline("1", None);
        queues_map.insert(id, MsgQueue::new(timeline));
        let queues = MessageQueues(queues_map);
        queues
    }

    pub fn to_event_struct(
        input: String,
        mut cache: &mut LruCache<String, i64>,
        mut queues: &mut MessageQueues,
        id: Uuid,
        timeline: Timeline,
    ) -> Event {
        redis_stream::process_messages(input, &mut None, &mut cache, &mut queues).unwrap();
        queues
            .oldest_msg_in_target_queue(id, timeline)
            .expect("In test")
    }
}

/// Parse using modified a modified version of Flodgatt's current function.
///
/// This version is modified to return a serde_json::Value instead of an Event to shows
/// the performance we would see if we used serde's built-in method for handling weakly
/// typed JSON instead of our own strongly typed struct.
mod flodgatt_parse_value {
    use flodgatt::{log_fatal, parse_client_request::Timeline};
    use lru::LruCache;
    use serde_json::Value;
    use std::{
        collections::{HashMap, VecDeque},
        time::Instant,
    };
    use uuid::Uuid;
    #[derive(Debug)]
    pub struct RedisMsg<'a> {
        pub raw: &'a str,
        pub cursor: usize,
        pub prefix_len: usize,
    }

    impl<'a> RedisMsg<'a> {
        pub fn from_raw(raw: &'a str, prefix_len: usize) -> Self {
            Self {
                raw,
                cursor: "*3\r\n".len(), //length of intro header
                prefix_len,
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

        pub fn extract_raw_timeline_and_message(&mut self) -> (String, Value) {
            let timeline = &self.next_field()[self.prefix_len..];
            let msg_txt = self.next_field();
            let msg_value: Value = serde_json::from_str(&msg_txt)
                .unwrap_or_else(|_| log_fatal!("Invalid JSON from Redis: {:?}", &msg_txt));
            (timeline.to_string(), msg_value)
        }
    }

    pub struct MsgQueue {
        pub timeline: Timeline,
        pub messages: VecDeque<Value>,
        _last_polled_at: Instant,
    }

    pub struct MessageQueues(HashMap<Uuid, MsgQueue>);
    impl std::ops::Deref for MessageQueues {
        type Target = HashMap<Uuid, MsgQueue>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl std::ops::DerefMut for MessageQueues {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl MessageQueues {
        pub fn oldest_msg_in_target_queue(
            &mut self,
            id: Uuid,
            timeline: Timeline,
        ) -> Option<Value> {
            self.entry(id)
                .or_insert_with(|| MsgQueue::new(timeline))
                .messages
                .pop_front()
        }
    }

    impl MsgQueue {
        pub fn new(timeline: Timeline) -> Self {
            MsgQueue {
                messages: VecDeque::new(),
                _last_polled_at: Instant::now(),
                timeline,
            }
        }
    }

    pub fn process_msg(
        raw_utf: String,
        namespace: &Option<String>,
        hashtag_id_cache: &mut LruCache<String, i64>,
        queues: &mut MessageQueues,
    ) {
        // Only act if we have a full message (end on a msg boundary)
        if !raw_utf.ends_with("}\r\n") {
            return;
        };
        let prefix_to_skip = match namespace {
            Some(namespace) => format!("{}:timeline:", namespace),
            None => "timeline:".to_string(),
        };

        let mut msg = RedisMsg::from_raw(&raw_utf, prefix_to_skip.len());

        while !msg.raw.is_empty() {
            let command = msg.next_field();
            match command.as_str() {
                "message" => {
                    let (raw_timeline, msg_value) = msg.extract_raw_timeline_and_message();
                    let hashtag = hashtag_from_timeline(&raw_timeline, hashtag_id_cache);
                    let timeline = Timeline::from_redis_raw_timeline(&raw_timeline, hashtag);
                    for msg_queue in queues.values_mut() {
                        if msg_queue.timeline == timeline {
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
                cmd => panic!("Invariant violation: {} is unexpected Redis output", cmd),
            };
            msg = RedisMsg::from_raw(&msg.raw[msg.cursor..], msg.prefix_len);
        }
    }

    fn hashtag_from_timeline(
        raw_timeline: &str,
        hashtag_id_cache: &mut LruCache<String, i64>,
    ) -> Option<i64> {
        if raw_timeline.starts_with("hashtag") {
            let tag_name = raw_timeline
                .split(':')
                .nth(1)
                .unwrap_or_else(|| log_fatal!("No hashtag found in `{}`", raw_timeline))
                .to_string();

            let tag_id = *hashtag_id_cache
                .get(&tag_name)
                .unwrap_or_else(|| log_fatal!("No cached id for `{}`", tag_name));
            Some(tag_id)
        } else {
            None
        }
    }
    pub fn setup() -> (LruCache<String, i64>, MessageQueues, Uuid, Timeline) {
        let cache: LruCache<String, i64> = LruCache::new(1000);
        let mut queues_map = HashMap::new();
        let id = Uuid::default();
        let timeline = Timeline::from_redis_raw_timeline("1", None);
        queues_map.insert(id, MsgQueue::new(timeline));
        let queues = MessageQueues(queues_map);
        (cache, queues, id, timeline)
    }

    pub fn to_json_value(
        input: String,
        mut cache: &mut LruCache<String, i64>,
        mut queues: &mut MessageQueues,
        id: Uuid,
        timeline: Timeline,
    ) -> Value {
        process_msg(input, &None, &mut cache, &mut queues);
        queues
            .oldest_msg_in_target_queue(id, timeline)
            .expect("In test")
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let input = ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS.to_string(); //INPUT.to_string();
    let mut group = c.benchmark_group("Parse redis RESP array");

    // group.bench_function("parse to Value with a regex", |b| {
    //     b.iter(|| regex_parse::to_json_value(black_box(input.clone())))
    // });
    group.bench_function("parse to Value inline", |b| {
        b.iter(|| parse_inline::to_json_value(black_box(input.clone())))
    });
    let (mut cache, mut queues, id, timeline) = flodgatt_parse_value::setup();
    group.bench_function("parse to Value using Flodgatt functions", |b| {
        b.iter(|| {
            black_box(flodgatt_parse_value::to_json_value(
                black_box(input.clone()),
                black_box(&mut cache),
                black_box(&mut queues),
                black_box(id),
                black_box(timeline),
            ))
        })
    });
    let mut queues = flodgatt_parse_event::setup();
    group.bench_function("parse to Event using Flodgatt functions", |b| {
        b.iter(|| {
            black_box(flodgatt_parse_event::to_event_struct(
                black_box(input.clone()),
                black_box(&mut cache),
                black_box(&mut queues),
                black_box(id),
                black_box(timeline),
            ))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
