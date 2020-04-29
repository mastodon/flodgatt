use super::{super::*, *};
use checked_event::{
    account::{Account, Field},
    tag::Tag,
    visibility::Visibility::*,
    CheckedEvent::*,
    *,
};
use std::fs;

#[test]
fn parse_redis_msg_to_event() -> Result<(), Box<dyn std::error::Error>> {
    let mut test_num = 1;

    let output = vec![
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_001.rs"
        )),
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_002.rs"
        )),
    ];

    while let (Ok(input), Some(output)) = (
        fs::read_to_string(format!("test_data/msg.event_txt_{:03}.txt", test_num)),
        output.get(test_num - 1),
    ) {
        println!("parsing `{:03}.resp`", test_num);
        test_num += 1;

        let event = Event::try_from(input)?;
        println!("{:#?}", event);

        assert_eq!(&event, output);
    }
    assert!(test_num > 1);

    Ok(())
}
