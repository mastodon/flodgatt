use super::*;
use crate::config;
use crate::response::event::checked_event::{
    account::{Account, Field},
    status::attachment::{Attachment, AttachmentType::*},
    status::Status,
    tag::Tag,
    visibility::Visibility::*,
    CheckedEvent::*,
};
use crate::Id;
use serde_json::json;
use std::fs;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn input(i: usize) -> Vec<u8> {
    fs::read_to_string(format!("test_data/redis_input_{:03}.resp", i))
        .expect("test input not found")
        .as_bytes()
        .to_vec()
}
fn output(i: usize) -> Arc<Event> {
    vec![
        Arc::new(include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_001.rs"
        ))),
        Arc::new(include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_002.rs"
        ))),
        Arc::new(include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_003.rs"
        ))),
        Arc::new(include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_004.rs"
        ))),
        Arc::new(include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_005.rs"
        ))),
        Arc::new(include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/event_006.rs"
        ))),
    ][i]
        .clone()
}

#[test]
fn manager_poll_matches_six_events() -> TestResult {
    let mut manager = Manager::try_from(&config::Redis::default())?;
    for i in 1..=6 {
        manager.redis_conn.add(&input(i));
    }
    let mut i = 0;
    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx = (0, manager.unread_idx.1 + len);
        while let Ok(Async::Ready(Some((_tl, event)))) = manager.poll() {
            println!("Parsing Event #{:03}", i + 1);
            assert_eq!(event, output(i));
            i += 1;
        }
    }
    Ok(assert_eq!(i, 6))
}

#[test]
fn manager_poll_handles_non_utf8() -> TestResult {
    let mut manager = Manager::try_from(&config::Redis::default())?;
    let mut input_txt = Vec::new();
    for i in 1..=6 {
        input_txt.extend_from_slice(&input(i))
    }

    let invalid_idx = str::from_utf8(&input_txt)?
        .chars()
        .take_while(|char| char.len_utf8() == 1)
        .collect::<Vec<_>>()
        .len()
        + 1;

    manager.redis_conn.add(&input_txt[..invalid_idx]);

    let mut i = 0;
    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(Some((_tl, event)))) = manager.poll() {
            println!("Parsing Event #{:03}", i + 1);
            assert_eq!(event, output(i));
            i += 1;
        }
    }

    manager.redis_conn.add(&input_txt[invalid_idx..]);

    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(Some((_tl, event)))) = manager.poll() {
            println!("Parsing Event #{:03}", i + 1);
            assert_eq!(event, output(i));
            i += 1;
        }
    }

    Ok(assert_eq!(i, 6))
}

#[test]
fn manager_poll_matches_six_events_in_batches() -> TestResult {
    let mut manager = Manager::try_from(&config::Redis::default())?;
    for i in 1..=3 {
        manager.redis_conn.add(&input(i))
    }
    let mut i = 0;
    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(Some((_tl, event)))) = manager.poll() {
            println!("Parsing Event #{:03}", i + 1);
            assert_eq!(event, output(i));
            i += 1;
        }
    }

    for i in 4..=6 {
        manager.redis_conn.add(&input(i));
    }
    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(Some((_tl, event)))) = manager.poll() {
            println!("Parsing Event #{:03}", i + 1);
            assert_eq!(event, output(i));
            i += 1;
        }
    }
    Ok(assert_eq!(i, 6))
}

#[test]
fn manager_poll_handles_non_events() -> TestResult {
    let mut manager = Manager::try_from(&config::Redis::default())?;
    for i in 1..=6 {
        manager.redis_conn.add(&input(i));
        manager
            .redis_conn
            .add(b"*3\r\n$9\r\nsubscribe\r\n$12\r\ntimeline:308\r\n:1\r\n");
    }
    let mut i = 0;

    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(msg)) = manager.poll() {
            if let Some((_tl, event)) = msg {
                println!("Parsing Event #{:03}", i + 1);
                assert_eq!(event, output(i));
                i += 1;
            }
        }
    }
    Ok(assert_eq!(i, 6))
}

#[test]
fn manager_poll_handles_partial_events() -> TestResult {
    let mut manager = Manager::try_from(&config::Redis::default())?;
    for i in 1..=3 {
        manager.redis_conn.add(&input(i));
    }
    manager.redis_conn.add(&input(4)[..50]);
    let mut i = 0;

    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(msg)) = manager.poll() {
            if let Some((_tl, event)) = msg {
                println!("Parsing Event #{:03}", i + 1);
                assert_eq!(event, output(i));
                i += 1;
            }
        }
    }
    assert_eq!(i, 3);

    manager.redis_conn.add(&input(4)[50..]);
    manager.redis_conn.add(&input(5));
    manager.redis_conn.add(&input(6));
    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(msg)) = manager.poll() {
            if let Some((_tl, event)) = msg {
                println!("Parsing Event #{:03}", i + 1);
                assert_eq!(event, output(i));
                i += 1;
            }
        }
    }

    Ok(assert_eq!(i, 6))
}

#[test]
fn manager_poll_handles_full_channel() -> TestResult {
    let mut manager = Manager::try_from(&config::Redis::default())?;
    for i in 1..=6 {
        manager.redis_conn.add(&input(i));
    }
    let (mut i, channel_full) = (0, 3);
    'outer: loop {
        while let Ok(Async::Ready(Some(n))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
            manager.unread_idx.1 += n;
            while let Ok(Async::Ready(msg)) = manager.poll() {
                if let Some((_tl, event)) = msg {
                    println!("Parsing Event #{:03}", i + 1);
                    assert_eq!(event, output(i));
                    i += 1;
                }
                // Simulates a `ChannelFull` error after sending `channel_full` msgs
                if i == channel_full {
                    break 'outer;
                }
            }
        }
    }

    let _rewind = (|| {
        manager.rewind_to_prev_msg();
        i -= 1;
    })();

    while let Ok(Async::Ready(Some(len))) = manager.redis_conn.poll_redis(manager.unread_idx.1) {
        manager.unread_idx.1 += len;
        while let Ok(Async::Ready(msg)) = manager.poll() {
            if let Some((_tl, event)) = msg {
                println!("Parsing Event #{:03}", i + 1);
                assert_eq!(event, output(i));
                i += 1;
            }
        }
    }

    Ok(assert_eq!(i, 6))
}
