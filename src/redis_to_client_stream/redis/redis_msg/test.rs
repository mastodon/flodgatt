use super::*;

#[test]
fn parse_redis_subscribe() -> Result<(), RedisParseErr> {
    let input = "*3\r\n$9\r\nsubscribe\r\n$15\r\ntimeline:public\r\n:1\r\n";

    let r_subscribe = match RedisParseOutput::try_from(input) {
        Ok(NonMsg(leftover)) => leftover,
        Ok(Msg(msg)) => panic!("unexpectedly got a msg: {:?}", msg),
        Err(e) => panic!("Error in parsing subscribe command: {:?}", e),
    };
    assert!(r_subscribe.is_empty());

    Ok(())
}

#[test]
fn parse_redis_detects_non_newline() -> Result<(), RedisParseErr> {
    let input =
        "*3QQ$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$38\r\n{\"event\":\"delete\",\"payload\":\"1038647\"}\r\n";

    match RedisParseOutput::try_from(input) {
        Ok(NonMsg(leftover)) => panic!(
            "Parsed an invalid msg as a non-msg.\nInput `{}` parsed to NonMsg({:?})",
            &input, leftover
        ),
        Ok(Msg(msg)) => panic!(
            "Parsed an invalid msg as a msg.\nInput `{:?}` parsed to {:?}",
            &input, msg
        ),
        Err(_) => (), // should err
    };

    Ok(())
}

#[test]
fn parse_redis_msg() -> Result<(), RedisParseErr> {
    let input =
        "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$38\r\n{\"event\":\"delete\",\"payload\":\"1038647\"}\r\n";

    let r_msg = match RedisParseOutput::try_from(input) {
        Ok(NonMsg(leftover)) => panic!(
            "Parsed a msg as a non-msg.\nInput `{}` parsed to NonMsg({:?})",
            &input, leftover
        ),
        Ok(Msg(msg)) => msg,
        Err(e) => panic!("Error in parsing subscribe command: {:?}", e),
    };

    assert!(r_msg.leftover_input.is_empty());
    assert_eq!(r_msg.timeline_txt, "timeline:308");
    assert_eq!(r_msg.event_txt, r#"{"event":"delete","payload":"1038647"}"#);
    Ok(())
}

#[test]
fn parse_long_redis_msg() -> Result<(), RedisParseErr> {
    let input = ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS;

    let r_msg = match RedisParseOutput::try_from(input) {
        Ok(NonMsg(leftover)) => panic!(
            "Parsed a msg as a non-msg.\nInput `{}` parsed to NonMsg({:?})",
            &input, leftover
        ),
        Ok(Msg(msg)) => msg,
        Err(e) => panic!("Error in parsing subscribe command: {:?}", e),
    };

    assert!(r_msg.leftover_input.is_empty());
    assert_eq!(r_msg.timeline_txt, "timeline:1");
    Ok(())
}

const ONE_MESSAGE_FOR_THE_USER_TIMLINE_FROM_REDIS: &str = "*3\r\n$7\r\nmessage\r\n$10\r\ntimeline:1\r\n$3790\r\n{\"event\":\"update\",\"payload\":{\"id\":\"102775370117886890\",\"created_at\":\"2019-09-11T18:42:19.000Z\",\"in_reply_to_id\":null,\"in_reply_to_account_id\":null,\"sensitive\":false,\"spoiler_text\":\"\",\"visibility\":\"unlisted\",\"language\":\"en\",\"uri\":\"https://mastodon.host/users/federationbot/statuses/102775346916917099\",\"url\":\"https://mastodon.host/@federationbot/102775346916917099\",\"replies_count\":0,\"reblogs_count\":0,\"favourites_count\":0,\"favourited\":false,\"reblogged\":false,\"muted\":false,\"content\":\"<p>Trending tags:<br><a href=\\\"https://mastodon.host/tags/neverforget\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>neverforget</span></a><br><a href=\\\"https://mastodon.host/tags/4styles\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>4styles</span></a><br><a href=\\\"https://mastodon.host/tags/newpipe\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>newpipe</span></a><br><a href=\\\"https://mastodon.host/tags/uber\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>uber</span></a><br><a href=\\\"https://mastodon.host/tags/mercredifiction\\\" class=\\\"mention hashtag\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">#<span>mercredifiction</span></a></p>\",\"reblog\":null,\"account\":{\"id\":\"78\",\"username\":\"federationbot\",\"acct\":\"federationbot@mastodon.host\",\"display_name\":\"Federation Bot\",\"locked\":false,\"bot\":false,\"created_at\":\"2019-09-10T15:04:25.559Z\",\"note\":\"<p>Hello, I am mastodon.host official semi bot.</p><p>Follow me if you want to have some updates on the view of the fediverse from here ( I only post unlisted ). </p><p>I also randomly boost one of my followers toot every hour !</p><p>If you don\'t feel confortable with me following you, tell me: unfollow  and I\'ll do it :)</p><p>If you want me to follow you, just tell me follow ! </p><p>If you want automatic follow for new users on your instance and you are an instance admin, contact me !</p><p>Other commands are private :)</p>\",\"url\":\"https://mastodon.host/@federationbot\",\"avatar\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"avatar_static\":\"https://instance.codesections.com/system/accounts/avatars/000/000/078/original/d9e2be5398629cf8.jpeg?1568127863\",\"header\":\"https://instance.codesections.com/headers/original/missing.png\",\"header_static\":\"https://instance.codesections.com/headers/original/missing.png\",\"followers_count\":16636,\"following_count\":179532,\"statuses_count\":50554,\"emojis\":[],\"fields\":[{\"name\":\"More stats\",\"value\":\"<a href=\\\"https://mastodon.host/stats.html\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/stats.html</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"More infos\",\"value\":\"<a href=\\\"https://mastodon.host/about/more\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\"><span class=\\\"invisible\\\">https://</span><span class=\\\"\\\">mastodon.host/about/more</span><span class=\\\"invisible\\\"></span></a>\",\"verified_at\":null},{\"name\":\"Owner/Friend\",\"value\":\"<span class=\\\"h-card\\\"><a href=\\\"https://mastodon.host/@gled\\\" class=\\\"u-url mention\\\" rel=\\\"nofollow noopener\\\" target=\\\"_blank\\\">@<span>gled</span></a></span>\",\"verified_at\":null}]},\"media_attachments\":[],\"mentions\":[],\"tags\":[{\"name\":\"4styles\",\"url\":\"https://instance.codesections.com/tags/4styles\"},{\"name\":\"neverforget\",\"url\":\"https://instance.codesections.com/tags/neverforget\"},{\"name\":\"mercredifiction\",\"url\":\"https://instance.codesections.com/tags/mercredifiction\"},{\"name\":\"uber\",\"url\":\"https://instance.codesections.com/tags/uber\"},{\"name\":\"newpipe\",\"url\":\"https://instance.codesections.com/tags/newpipe\"}],\"emojis\":[],\"card\":null,\"poll\":null},\"queued_at\":1568227693541}\r\n";
