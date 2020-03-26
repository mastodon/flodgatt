//! Methods for parsing input in the small subset of the Redis Serialization Protocol we
//! support.
//!
//! Every message Flodgatt receives from Redis is a Redis Array; the elements in the array
//! will be either Bulk Strings or Integers (as Redis defines those terms).  See the
//! [Redis protocol documentation](https://redis.io/topics/protocol) for details. A raw
//! message might look slightly like this (simplified, with line brakes added between
//! fields):
//!
//! ```text
//! *3\r\n
//! $7\r\nmessage\r\n
//! $10\r\ntimeline:4\r\n
//! $1386\r\n{\"event\":\"update\",\"payload\"...\"queued_at\":1569623342825}\r\n
//! ```
//!
//! Read that as: an array with three elements: the first element is a bulk string with
//! three characters, the second is a bulk string with ten characters, and the third is a
//! bulk string with 1,386 characters.

use crate::{log_fatal, messages::Event, parse_client_request::Timeline};
use lru::LruCache;
type Parser<'a, Item> = Result<(Item, &'a str), ParseErr>;
#[derive(Debug)]
pub enum ParseErr {
    Incomplete,
    Unrecoverable,
}
use ParseErr::*;

/// A message that has been parsed from an incoming raw message from Redis.
#[derive(Debug, Clone)]
pub enum RedisMsg {
    EventMsg(Timeline, Event),
    SubscriptionMsg,
}

type Hashtags = LruCache<String, i64>;
impl RedisMsg {
    pub fn from_raw<'a>(input: &'a str, cache: &mut Hashtags, prefix: usize) -> Parser<'a, Self> {
        // No need to parse the Redis Array header, just skip it
        let input = input.get("*3\r\n".len()..).ok_or(Incomplete)?;
        let (command, rest) = parse_redis_bulk_string(&input)?;
        match command {
            "message" => {
                // Messages look like;
                // $10\r\ntimeline:4\r\n
                // $1386\r\n{\"event\":\"update\",\"payload\"...\"queued_at\":1569623342825}\r\n
                let (raw_timeline, rest) = parse_redis_bulk_string(&rest)?;
                let (msg_txt, rest) = parse_redis_bulk_string(&rest)?;

                let raw_timeline = &raw_timeline.get(prefix..).ok_or(Unrecoverable)?;
                let event: Event = serde_json::from_str(&msg_txt).unwrap();
                let hashtag = hashtag_from_timeline(&raw_timeline, cache);
                let timeline = Timeline::from_redis_raw_timeline(&raw_timeline, hashtag);
                Ok((Self::EventMsg(timeline, event), rest))
            }
            "subscribe" | "unsubscribe" => {
                // subscription statuses look like:
                // $14\r\ntimeline:local\r\n
                // :47\r\n
                let (_raw_timeline, rest) = parse_redis_bulk_string(&rest)?;
                let (_number_of_subscriptions, rest) = parse_redis_int(&rest)?;
                Ok((Self::SubscriptionMsg, &rest))
            }
            _cmd => Err(Incomplete)?,
        }
    }
}

/// Parse a Redis bulk string and return the content of that string and the unparsed remainder.
///
/// All bulk strings have the format `$[LENGTH_OF_ITEM_BODY]\r\n[ITEM_BODY]\r\n`
fn parse_redis_bulk_string(input: &str) -> Parser<&str> {
    let input = &input.get("$".len()..).ok_or(Incomplete)?;
    let (field_len, rest) = parse_redis_length(input)?;
    let field_content = rest.get(..field_len).ok_or(Incomplete)?;
    Ok((field_content, &rest[field_len + "\r\n".len()..]))
}

fn parse_redis_int(input: &str) -> Parser<usize> {
    let input = &input.get(":".len()..).ok_or(Incomplete)?;
    let (number, rest_with_newline) = parse_number_at(input)?;
    let rest = &rest_with_newline.get("\r\n".len()..).ok_or(Incomplete)?;
    Ok((number, rest))
}

/// Return the value of a Redis length (for an array or bulk string) and the unparsed remainder
fn parse_redis_length(input: &str) -> Parser<usize> {
    let (number, rest_with_newline) = parse_number_at(input)?;
    let rest = &rest_with_newline.get("\r\n".len()..).ok_or(Incomplete)?;
    Ok((number, rest))
}

fn parse_number_at(input: &str) -> Parser<usize> {
    let number_len = input
        .chars()
        .position(|c| !c.is_numeric())
        .ok_or(Unrecoverable)?;
    let number = input[..number_len].parse().map_err(|_| Unrecoverable)?;
    let rest = &input.get(number_len..).ok_or(Incomplete)?;
    Ok((number, rest))
}
fn hashtag_from_timeline(raw_timeline: &str, hashtag_id_cache: &mut Hashtags) -> Option<i64> {
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
