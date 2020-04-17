use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flodgatt::{
    event::*,
    request::{Content::*, Reach::*, Stream::*, Timeline},
    response::{RedisMsg, RedisParseOutput},
};
use lru::LruCache;
use std::convert::TryFrom;

fn parse_long_redis_input<'a>(input: &'a str) -> RedisMsg<'a> {
    if let RedisParseOutput::Msg(msg) = RedisParseOutput::try_from(input).unwrap() {
        assert_eq!(msg.timeline_txt, "timeline:1");
        msg
    } else {
        panic!()
    }
}

fn parse_to_timeline(msg: RedisMsg) -> Timeline {
    let trimmed_tl_txt = &msg.timeline_txt["timeline:".len()..];
    let tl = Timeline::from_redis_text(trimmed_tl_txt, &mut LruCache::new(1000)).unwrap();
    assert_eq!(tl, Timeline(User(Id(1)), Federated, All));
    tl
}
fn parse_to_checked_event(msg: RedisMsg) -> Event {
    Event::TypeSafe(serde_json::from_str(msg.event_txt).unwrap())
}

fn parse_to_dyn_event(msg: RedisMsg) -> Event {
    Event::Dynamic(serde_json::from_str(msg.event_txt).unwrap())
}

fn redis_msg_to_event_string(msg: RedisMsg) -> String {
    msg.event_txt.to_string()
}

fn string_to_checked_event(event_txt: &String) -> Event {
    Event::TypeSafe(serde_json::from_str(event_txt).unwrap())
}

fn criterion_benchmark(c: &mut Criterion) {
    let input = ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS;
    let mut group = c.benchmark_group("Parse redis RESP array");

    group.bench_function("parse redis input to RedisMsg", |b| {
        b.iter(|| black_box(parse_long_redis_input(input)))
    });

    let msg = parse_long_redis_input(input);
    group.bench_function("parse RedisMsg to Timeline", |b| {
        b.iter(|| black_box(parse_to_timeline(msg.clone())))
    });

    group.bench_function("parse RedisMsg -> DynamicEvent", |b| {
        b.iter(|| black_box(parse_to_dyn_event(msg.clone())))
    });

    group.bench_function("parse RedisMsg -> CheckedEvent", |b| {
        b.iter(|| black_box(parse_to_checked_event(msg.clone())))
    });

    group.bench_function("parse RedisMsg -> String -> CheckedEvent", |b| {
        b.iter(|| {
            let txt = black_box(redis_msg_to_event_string(msg.clone()));
            black_box(string_to_checked_event(&txt));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

const ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS: &str = "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:1\r\n$3790\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102775370117886890\",\"created_at\":\"2019-09-11T18:42:19.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"unlisted\",\"language\":\"en\",\"uri\":\"https://mastodon.host/users/federationbot/statuses/102775346916917099\",\"url\":\"https://mastodon.host/@federationbot/102775346916917099\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p>Trending tags:<br><a href=\\\"https://mastodon.host/tags/neverforget\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>neverforget</span></a><br><a href=\\\"https://mastodon.host/tags/4styles\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>4styles</span></a><br><a href=\\\"https://mastodon.host/tags/newpipe\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>newpipe</span></a><br><a href=\\\"https://mastodon.host/tags/uber\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>uber</span></a><br><a href=\\\"https://mastodon.host/tags/mercredifiction\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>mercredifiction</span></a></p>\",\"reblog\":null,\"account\":{\"id\":\"78\",\"username\":\"federationbot\",\"acct\":\"federationbot@mastodon.host\",\"display_name\":\"Federation Bot\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-09-10T15:04:25.559Z\",\"note\":\"<p>Hello, I am mastodon.host official semi bot.</p><p>Follow me if you want to have some updates on the view of the fediverse from here ( I only post unlisted ). </p><p>I also randomly boost one of my followers toot every hour !</p><p>If you don\'t feel confortable with me following you, tell me: unfollow  and I\'ll do it :)</p><p>If you want me to follow you, just tell me follow ! </p><p>If you want automatic follow for new users on your instance and you are an instance admin, contact me !</p><p>Other commands are private :)</p>\",\"url\":\"https://mastodon.host/@federationbot\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":16636,\"following_count\":179532,\"statuses_count\":50554,\"emojis\":[],\"fields\":[{\"name\":\"More stats\",\"value\":\"<a href=\\\"https://mastodon.host/stats.html\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/stats.html</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"More infos\",\"value\":\"<a href=\\\"https://mastodon.host/about/more\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/about/more</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Owner/Friend\",\"value\":\"<span class=\\\"h-card\\\"><a href=\\\"https://mastodon.host/@gled\\\" class=\\\"u-url mention\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">@<span>gled</span></a></span>\",\"verified_at\":null}]},\"media_attachments\":[],\"mentions\":[],\"tags\":[{\"name\":\"4styles\",\"url\":\"https://instance.codesections.com/tags/4styles\"},{\"name\":\"neverforget\",\"url\":\"https://instance.codesections.com/tags/neverforget\"},{\"name\":\"mercredifiction\",\"url\":\"https://instance.codesections.com/tags/mercredifiction\"},{\"name\":\"uber\",\"url\":\"https://instance.codesections.com/tags/uber\"},{\"name\":\"newpipe\",\"url\":\"https://instance.codesections.com/tags/newpipe\"}],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1568227693541}\r\n";
