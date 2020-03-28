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

use crate::{
    err::{RedisParseErr, TimelineErr},
    messages::Event,
    parse_client_request::Timeline,
};
use lru::LruCache;
use std::str;

#[derive(Debug, Clone)]
pub struct RedisBytes<'a>(&'a [u8]);

#[derive(Debug, Clone)]
pub struct RedisUtf8<'a> {
    valid_utf8: &'a str,
    leftover_bytes: RedisBytes<'a>,
}

#[derive(Debug, Clone)]
pub struct RedisStructuredText<'a> {
    parsed_reply: RedisDataType<'a>,
    leftover_input: RedisUtf8<'a>,
}

#[derive(Debug, Clone)]
pub struct RedisMessage<'a> {
    timeline_txt: &'a str,
    event_txt: &'a str,
    leftover_input: RedisUtf8<'a>,
}

#[derive(Debug, Clone)]
pub struct RedisParsed<'a> {
    pub timeline: Timeline,
    pub event: Event,
    pub leftover_input: RedisUtf8<'a>,
}

#[derive(Debug, Clone)]
enum RedisDataType<'a> {
    RedisArray(Vec<RedisDataType<'a>>),
    BulkString(&'a str),
    Integer(usize),
    Uninitilized,
}

impl<'a> RedisBytes<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }
    pub fn into_redis_utf8(self) -> RedisUtf8<'a> {
        RedisUtf8::from(self)
    }
}

impl<'a> From<RedisBytes<'a>> for RedisUtf8<'a> {
    fn from(val: RedisBytes<'a>) -> Self {
        match str::from_utf8(val.0) {
            Ok(valid_utf8) => Self {
                valid_utf8,
                leftover_bytes: RedisBytes("".as_bytes()),
            },
            Err(e) => {
                let (valid, after_valid) = val.0.split_at(e.valid_up_to());
                Self {
                    valid_utf8: str::from_utf8(valid).expect("Guaranteed by `.valid_up_to`"),
                    leftover_bytes: RedisBytes(after_valid),
                }
            }
        }
    }
}

use RedisDataType::*;
type RedisParser<'a, Item> = Result<Item, RedisParseErr>;
impl<'a> RedisUtf8<'a> {
    pub fn try_into_redis_structured_text(self) -> Result<RedisStructuredText<'a>, RedisParseErr> {
        Self::new_try_from(self.valid_utf8)
    }

    pub fn as_leftover_bytes(&self) -> Vec<u8> {
        [self.valid_utf8.as_bytes(), self.leftover_bytes.0].concat()
    }

    fn from_str(valid_utf8: &'a str) -> Self {
        Self {
            valid_utf8,
            leftover_bytes: RedisBytes("".as_bytes()),
        }
    }

    fn parse_number_at(input: &'a str) -> RedisParser<(usize, &'a str)> {
        let number_len = input
            .chars()
            .position(|c| !c.is_numeric())
            .ok_or(Unrecoverable)?;
        let number = input[..number_len].parse().map_err(|_| Unrecoverable)?;
        let rest = input.get(number_len..).ok_or(Incomplete)?;
        Ok((number, rest))
    }
    /// Parse a Redis bulk string and return the content of that string and the unparsed remainder.
    ///
    /// All bulk strings have the format `$[LENGTH_OF_ITEM_BODY]\r\n[ITEM_BODY]\r\n`
    fn parse_redis_bulk_string(input: &'a str) -> RedisParser<RedisStructuredText> {
        let (field_len, rest) = parse_redis_length(input)?;
        let field_content = rest.get(..field_len).ok_or(Incomplete)?;
        Ok(RedisStructuredText {
            parsed_reply: BulkString(field_content),
            leftover_input: RedisUtf8::from_str(&rest[field_len + "\r\n".len()..]),
        })
    }

    fn parse_redis_int(input: &'a str) -> RedisParser<RedisStructuredText> {
        let (number, rest_with_newline) = parse_number_at(input)?;
        let leftover_utf8 =
            RedisUtf8::from_str(rest_with_newline.get("\r\n".len()..).ok_or(Incomplete)?);
        Ok(RedisStructuredText {
            parsed_reply: Integer(number),
            leftover_input: leftover_utf8,
        })
    }

    fn parse_redis_array(input: &'a str) -> RedisParser<RedisStructuredText> {
        let (number_of_elements, rest) = Self::parse_number_at(input)?;
        let mut inner = Vec::with_capacity(number_of_elements);
        let mut leftover_utf8 = RedisUtf8::from_str(rest.get("\r\n".len()..).ok_or(Incomplete)?);

        inner.resize(number_of_elements, RedisDataType::Uninitilized);

        for i in (0..number_of_elements).rev() {
            let next_el = Self::new_try_from(leftover_utf8.valid_utf8)?;
            leftover_utf8 = next_el.leftover_input;
            inner[i] = next_el.parsed_reply;
        }
        Ok(RedisStructuredText {
            parsed_reply: RedisDataType::RedisArray(inner),
            leftover_input: leftover_utf8,
        })
    }

    fn new_try_from(input: &'a str) -> Result<RedisStructuredText, RedisParseErr> {
        if input.len() < 4 {
            Err(Incomplete)?
        };
        let (first_char, input) = input.split_at(1);
        match first_char {
            ":" => Self::parse_redis_int(input),
            "$" => Self::parse_redis_bulk_string(input),
            "*" => Self::parse_redis_array(input),
            _ => panic!("TODO: Error for unimplemented"),
        }
    }
}

use std::convert::{TryFrom, TryInto};
impl<'a> TryFrom<RedisDataType<'a>> for &'a str {
    type Error = RedisParseErr;

    fn try_from(val: RedisDataType<'a>) -> Result<Self, Self::Error> {
        match val {
            RedisDataType::BulkString(inner) => Ok(inner),
            _ => Err(Unrecoverable),
        }
    }
}

impl<'a> RedisStructuredText<'a> {
    pub fn try_into_redis_message(self) -> Result<RedisMessage<'a>, RedisParseErr> {
        if let RedisDataType::RedisArray(mut redis_strings) = self.parsed_reply {
            let command = redis_strings.pop().expect("TODO").try_into()?;

            match command {
                // subscription statuses look like:
                // $14\r\ntimeline:local\r\n
                // :47\r\n
                "subscribe" | "unsubscribe" => panic!("TODO: skip"),
                // Messages look like;
                // $10\r\ntimeline:4\r\n
                // $1386\r\n{\"event\":\"update\",\"payload\"...\"queued_at\":1569623342825}\r\n
                "message" => Ok(RedisMessage {
                    timeline_txt: redis_strings.pop().expect("TODO").try_into()?,
                    event_txt: redis_strings.pop().expect("TODO").try_into()?,
                    leftover_input: self.leftover_input,
                }),
                _cmd => Err(Incomplete)?,
            }
        } else {
            panic!("TODO");
        }
    }
    pub fn try_into_parsed(
        self,
        cache: &mut LruCache<String, i64>,
        namespace: &Option<String>,
    ) -> Result<RedisParsed<'a>, RedisParseErr> {
        if let RedisDataType::RedisArray(mut redis_strings) = self.parsed_reply {
            let command = redis_strings.pop().expect("TODO").try_into()?;

            match command {
                "subscribe" | "unsubscribe" => {
                    // subscription statuses look like:
                    // $14\r\ntimeline:local\r\n
                    // :47\r\n
                    panic!("TODO: skip");
                }
                "message" => {
                    // Messages look like;
                    // $10\r\ntimeline:4\r\n
                    // $1386\r\n{\"event\":\"update\",\"payload\"...\"queued_at\":1569623342825}\r\n
                    let tl_txt = redis_strings.pop().expect("TODO").try_into()?;
                    let event_txt = redis_strings.pop().expect("TODO").try_into()?;
                    let timeline = match Timeline::from_redis_raw_timeline(tl_txt, cache, namespace)
                    {
                        Ok(timeline) => timeline,
                        Err(RedisNamespaceMismatch) => todo!(),
                        Err(InvalidInput) => todo!(),
                    };
                    let event: Event = serde_json::from_str(event_txt).expect("TODO");

                    Ok(RedisParsed {
                        timeline,
                        event,
                        leftover_input: self.leftover_input,
                    })
                }
                _cmd => Err(Incomplete)?,
            }
        } else {
            panic!("TODO");
        }
    }
}

type HashtagCache = LruCache<String, i64>;
impl<'a> RedisMessage<'a> {
    pub fn parse_timeline(
        &self,
        cache: &mut HashtagCache,
        namespace: &Option<String>,
    ) -> Result<Timeline, RedisParseErr> {
        match Timeline::from_redis_raw_timeline(self.timeline_txt, cache, namespace) {
            Ok(timeline) => Ok(timeline),
            Err(TimelineErr::RedisNamespaceMismatch) => todo!(),
            Err(TimelineErr::InvalidInput) => todo!(),
        }
    }
    pub fn parse_event(&self) -> Result<Event, RedisParseErr> {
        Ok(serde_json::from_str(self.event_txt).expect("TODO"))
    }
}

type Parser<'a, Item> = Result<(Item, &'a str), RedisParseErr>;

/// A message that has been parsed from an incoming raw message from Redis.
#[derive(Debug, Clone)]
pub enum RedisMsg<'a> {
    EventMsg { tl_txt: &'a str, event_txt: &'a str },
    SubscriptionMsg,
}

use RedisParseErr::*;

impl<'a> RedisMsg<'a> {
    pub fn from_raw(input: &'a str) -> Parser<'a, Self> {
        // No need to parse the Redis Array header, just skip it
        let input = input.get("*3\r\n".len()..).ok_or(Incomplete)?;
        let (command, rest) = parse_redis_bulk_string(&input)?;
        match command {
            "subscribe" | "unsubscribe" => {
                // subscription statuses look like:
                // $14\r\ntimeline:local\r\n
                // :47\r\n
                let (_raw_timeline, rest) = parse_redis_bulk_string(&rest)?;
                let (_number_of_subscriptions, rest) = parse_redis_int(&rest)?;
                Ok((RedisMsg::SubscriptionMsg, &rest))
            }
            "message" => {
                // Messages look like;
                // $10\r\ntimeline:4\r\n
                // $1386\r\n{\"event\":\"update\",\"payload\"...\"queued_at\":1569623342825}\r\n
                let (tl_txt, rest) = parse_redis_bulk_string(&rest)?;
                let (event_txt, rest) = parse_redis_bulk_string(&rest)?;

                Ok((RedisMsg::EventMsg { tl_txt, event_txt }, rest))
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_redis_int() -> Result<(), RedisParseErr> {
        let mut buffer = Vec::new();
        let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$45\r\n{\"event\":\"announcement.delete\",\"payload\":\"5\"}\r\n";

        let mut cache = LruCache::new(1000);
        let r_msg = RedisBytes(input.as_bytes())
            .into_redis_utf8()
            .try_into_redis_structured_text()?
            .try_into_redis_message()?;

        buffer.push(r_msg.leftover_input.as_leftover_bytes());

        let (timeline, event) = (
            r_msg.parse_timeline(&mut cache, &None)?,
            r_msg.parse_event()?,
        );

        Ok(())
    }
}
