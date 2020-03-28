use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;

/// Parse using Flodgatt's current functions
mod flodgatt_parse_event {
    use flodgatt::{
        err::RedisParseErr,
        messages::Event,
        parse_client_request::Timeline,
        redis_to_client_stream::redis_msg::{RedisBytes, RedisParsed},
        redis_to_client_stream::{process_messages, MessageQueues, MsgQueue},
    };
    use lru::LruCache;
    use std::collections::HashMap;
    use uuid::Uuid;

    /// One-time setup, not included in testing time.
    pub fn setup() -> (LruCache<String, i64>, MessageQueues, Uuid, Timeline) {
        let mut cache: LruCache<String, i64> = LruCache::new(1000);
        let mut queues_map = HashMap::new();
        let id = Uuid::default();
        let timeline =
            Timeline::from_redis_raw_timeline("timeline:1", &mut cache, &None).expect("In test");
        queues_map.insert(id, MsgQueue::new(timeline));
        let queues = MessageQueues(queues_map);
        (cache, queues, id, timeline)
    }

    pub fn to_event_struct(
        input: String,
        mut cache: &mut LruCache<String, i64>,
        mut queues: &mut MessageQueues,
        id: Uuid,
        timeline: Timeline,
    ) -> Event {
        process_messages(&input, &mut cache, &mut None, &mut queues);
        queues
            .oldest_msg_in_target_queue(id, timeline)
            .expect("In test")
    }

    pub fn mutistep(
        input: String,
        mut cache: &mut LruCache<String, i64>,
    ) -> Result<RedisParsed, RedisParseErr> {
        Ok(RedisBytes::new(input.as_bytes())
            .into_redis_utf8()
            .try_into_redis_reply()?
            .try_into_parsed(&mut cache, &None)?)
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    use flodgatt::redis_to_client_stream::redis_msg::RedisParsed;
    let input = ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS.to_string();
    let mut group = c.benchmark_group("Parse redis RESP array");

    let (mut cache, mut queues, id, timeline) = flodgatt_parse_event::setup();
    group.bench_function(
        "parse to Event using Flodgatt new, multi-step function",
        |b| {
            b.iter(|| {
                let RedisParsed(timeline, event) = black_box(
                    flodgatt_parse_event::mutistep(black_box(input.clone()), black_box(&mut cache))
                        .unwrap(),
                );

                for msg_queue in queues.values_mut() {
                    if msg_queue.timeline == timeline {
                        msg_queue.messages.push_back(event.clone());
                    }
                }
                queues
                    .oldest_msg_in_target_queue(id, timeline)
                    .expect("In test")
            })
        },
    );

    let (mut cache, mut queues, id, timeline) = flodgatt_parse_event::setup();
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

const ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS: &str = "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:1\r\n$3790\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102775370117886890\",\"created_at\":\"2019-09-11T18:42:19.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"unlisted\",\"language\":\"en\",\"uri\":\"https://mastodon.host/users/federationbot/statuses/102775346916917099\",\"url\":\"https://mastodon.host/@federationbot/102775346916917099\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p>Trending tags:<br><a href=\\\"https://mastodon.host/tags/neverforget\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>neverforget</span></a><br><a href=\\\"https://mastodon.host/tags/4styles\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>4styles</span></a><br><a href=\\\"https://mastodon.host/tags/newpipe\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>newpipe</span></a><br><a href=\\\"https://mastodon.host/tags/uber\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>uber</span></a><br><a href=\\\"https://mastodon.host/tags/mercredifiction\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>mercredifiction</span></a></p>\",\"reblog\":null,\"account\":{\"id\":\"78\",\"username\":\"federationbot\",\"acct\":\"federationbot@mastodon.host\",\"display_name\":\"Federation Bot\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-09-10T15:04:25.559Z\",\"note\":\"<p>Hello, I am mastodon.host official semi bot.</p><p>Follow me if you want to have some updates on the view of the fediverse from here ( I only post unlisted ). </p><p>I also randomly boost one of my followers toot every hour !</p><p>If you don\'t feel confortable with me following you, tell me: unfollow  and I\'ll do it :)</p><p>If you want me to follow you, just tell me follow ! </p><p>If you want automatic follow for new users on your instance and you are an instance admin, contact me !</p><p>Other commands are private :)</p>\",\"url\":\"https://mastodon.host/@federationbot\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":16636,\"following_count\":179532,\"statuses_count\":50554,\"emojis\":[],\"fields\":[{\"name\":\"More stats\",\"value\":\"<a href=\\\"https://mastodon.host/stats.html\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/stats.html</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"More infos\",\"value\":\"<a href=\\\"https://mastodon.host/about/more\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/about/more</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Owner/Friend\",\"value\":\"<span class=\\\"h-card\\\"><a href=\\\"https://mastodon.host/@gled\\\" class=\\\"u-url mention\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">@<span>gled</span></a></span>\",\"verified_at\":null}]},\"media_attachments\":[],\"mentions\":[],\"tags\":[{\"name\":\"4styles\",\"url\":\"https://instance.codesections.com/tags/4styles\"},{\"name\":\"neverforget\",\"url\":\"https://instance.codesections.com/tags/neverforget\"},{\"name\":\"mercredifiction\",\"url\":\"https://instance.codesections.com/tags/mercredifiction\"},{\"name\":\"uber\",\"url\":\"https://instance.codesections.com/tags/uber\"},{\"name\":\"newpipe\",\"url\":\"https://instance.codesections.com/tags/newpipe\"}],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1568227693541}\r\n";
