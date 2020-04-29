use super::*;
use std::fs;
use std::path;

#[test]
fn parse_redis_subscribe() -> Result<(), RedisParseErr> {
    let input = "*3\r\n$9\r\nsubscribe\r\n$15\r\ntimeline:public\r\n:1\r\n";

    let r_subscribe = match RedisParseOutput::try_from(input) {
        Ok(NonMsg(leftover)) => leftover,
        Ok(Msg(msg)) => panic!("unexpectedly got a msg: {:?}", msg),
        Err(e) => panic!("Error in parsing subscribe command: {}", e),
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
            "Parsed an invalid msg as a non-msg.\nInput `{}` parsed to NonMsg({})",
            &input, leftover
        ),
        Ok(Msg(msg)) => panic!(
            "Parsed an invalid msg as a msg.\nInput `{}` parsed to {:?}",
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
        Err(e) => panic!("Error in parsing subscribe command: {}", e),
    };

    assert!(r_msg.leftover_input.is_empty());
    assert_eq!(r_msg.timeline_txt, "timeline:308");
    assert_eq!(r_msg.event_txt, r#"{"event":"delete","payload":"1038647"}"#);
    Ok(())
}

#[test]
fn parse_long_redis_msg() -> Result<(), Box<dyn std::error::Error>> {
    let mut test_num = 1;
    while let (Ok(input), Ok(output)) = (
        fs::read_to_string(format!("test_data/redis_input_{:03}.resp", test_num)),
        fs::read_to_string(format!("test_data/msg.event_txt_{:03}.txt", test_num)),
    ) {
        println!("parsing `{:03}.resp`", test_num);
        test_num += 1;

        let r_msg = match RedisParseOutput::try_from(input.as_str()) {
            Ok(NonMsg(leftover)) => panic!(
                "Parsed a msg as a non-msg.\nInput `{}` parsed to NonMsg({:?})",
                &input, leftover
            ),
            Ok(Msg(msg)) => msg,
            Err(e) => panic!("Error in parsing Redis input: {}", e),
        };
        assert!(r_msg.leftover_input.is_empty());
        assert_eq!(r_msg.event_txt, output);

        assert_eq!(r_msg.timeline_txt, "timeline:public");
    }
    assert!(test_num > 1);

    Ok(())
}
