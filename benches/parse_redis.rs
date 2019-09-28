use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use flodgatt::redis_to_client_stream::redis_stream::RedisMsg;
use regex::Regex;
use serde_json::Value;

fn regex_parse(input: String) -> Vec<(String, Value)> {
    let mut output = Vec::new();
    if input.ends_with("}\r\n") {
        // Every valid message is tagged with the string `message`.  This means 3 things:
        //   1) We can discard everything before the first `message` (with `skip(1)`)
        //   2) We can split into separate messages by splitting on `message`
        //   3) We can use a regex that discards everything after the *first* valid
        //      message (since the next message will have a new `message` tag)

        let messages = input.as_str().split("message").skip(1);
        let regex = Regex::new(r"timeline:(?P<timeline>.*?)\r\n\$\d+\r\n(?P<value>.*?)\r\n")
            .expect("Hard-codded");
        for message in messages {
            let timeline =
                regex.captures(message).expect("Hard-coded timeline regex")["timeline"].to_string();

            let redis_msg: Value = serde_json::from_str(
                &regex.captures(message).expect("Hard-coded value regex")["value"],
            )
            .expect("Valid json");

            output.push((timeline, redis_msg));
        }
    }
    output
}

fn hand_parse(input: String) -> Vec<(String, Value)> {
    let mut output = Vec::new();
    if input.ends_with("}\r\n") {
        let end = 2;
        let (end, _) = print_next_str(end, &input);
        let (end, timeline) = print_next_str(end, &input);
        let (_, msg) = print_next_str(end, &input);
        let redis_msg: Value = serde_json::from_str(&msg).unwrap();
        output.push((timeline, redis_msg));
    }
    output
}

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

fn parse_with_stuct(input: String) -> Vec<(String, Value)> {
    let mut output = Vec::new();
    let mut incoming_raw_msg = input;

    while incoming_raw_msg.len() > 0 {
        let mut msg = RedisMsg::from_raw(incoming_raw_msg.clone());
        let command = msg.get_next_item();
        match command.as_str() {
            "message" => {
                let timeline = msg.get_next_item()["timeline:".len()..].to_string();
                let message: Value = serde_json::from_str(&msg.get_next_item()).unwrap();
                output.push((timeline, message));
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
        incoming_raw_msg = msg.raw[msg.cursor..].to_string();
    }
    output
}

fn criterion_benchmark(c: &mut Criterion) {
    let input =             "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:1\r\n$3790\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102775370117886890\",\"created_at\":\"2019-09-11T18:42:19.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"unlisted\",\"language\":\"en\",\"uri\":\"https://mastodon.host/users/federationbot/statuses/102775346916917099\",\"url\":\"https://mastodon.host/@federationbot/102775346916917099\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p>Trending tags:<br><a href=\\\"https://mastodon.host/tags/neverforget\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>neverforget</span></a><br><a href=\\\"https://mastodon.host/tags/4styles\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>4styles</span></a><br><a href=\\\"https://mastodon.host/tags/newpipe\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>newpipe</span></a><br><a href=\\\"https://mastodon.host/tags/uber\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>uber</span></a><br><a href=\\\"https://mastodon.host/tags/mercredifiction\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>mercredifiction</span></a></p>\",\"reblog\":null,\"account\":{\"id\":\"78\",\"username\":\"federationbot\",\"acct\":\"federationbot@mastodon.host\",\"display_name\":\"Federation Bot\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-09-10T15:04:25.559Z\",\"note\":\"<p>Hello, I am mastodon.host official semi bot.</p><p>Follow me if you want to have some updates on the view of the fediverse from here ( I only post unlisted ). </p><p>I also randomly boost one of my followers toot every hour !</p><p>If you don\'t feel confortable with me following you, tell me: unfollow  and I\'ll do it :)</p><p>If you want me to follow you, just tell me follow ! </p><p>If you want automatic follow for new users on your instance and you are an instance admin, contact me !</p><p>Other commands are private :)</p>\",\"url\":\"https://mastodon.host/@federationbot\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":16636,\"following_count\":179532,\"statuses_count\":50554,\"emojis\":[],\"fields\":[{\"name\":\"More stats\",\"value\":\"<a href=\\\"https://mastodon.host/stats.html\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/stats.html</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"More infos\",\"value\":\"<a href=\\\"https://mastodon.host/about/more\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/about/more</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Owner/Friend\",\"value\":\"<span class=\\\"h-card\\\"><a href=\\\"https://mastodon.host/@gled\\\" class=\\\"u-url mention\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">@<span>gled</span></a></span>\",\"verified_at\":null}]},\"media_attachments\":[],\"mentions\":[],\"tags\":[{\"name\":\"4styles\",\"url\":\"https://instance.codesections.com/tags/4styles\"},{\"name\":\"neverforget\",\"url\":\"https://instance.codesections.com/tags/neverforget\"},{\"name\":\"mercredifiction\",\"url\":\"https://instance.codesections.com/tags/mercredifiction\"},{\"name\":\"uber\",\"url\":\"https://instance.codesections.com/tags/uber\"},{\"name\":\"newpipe\",\"url\":\"https://instance.codesections.com/tags/newpipe\"}],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1568227693541}\r\n".to_string();

    let mut group = c.benchmark_group("Parse redis RESP array");
    group.bench_function("regex parse", |b| {
        b.iter(|| regex_parse(black_box(input.clone())))
    });
    group.bench_function("hand parse", |b| {
        b.iter(|| hand_parse(black_box(input.clone())))
    });
    group.bench_function("stuct parse", |b| {
        b.iter(|| parse_with_stuct(black_box(input.clone())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
