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
//! $7\r\n
//! message\r\n
//! $10\r\n
//! timeline:4\r\n
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
use std::{
    convert::{TryFrom, TryInto},
    str,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RedisBytes<'a>(&'a [u8]);

#[derive(Debug, Clone, PartialEq)]
pub struct RedisUtf8<'a> {
    valid_utf8: &'a str,
    leftover_bytes: RedisBytes<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedisStructuredText<'a> {
    parsed_reply: RedisDataType<'a>,
    pub leftover_input: RedisUtf8<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedisMessage<'a> {
    timeline_txt: &'a str,
    event_txt: &'a str,
    pub leftover_input: RedisUtf8<'a>,
}

#[derive(Debug, Clone, PartialEq)]
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
use RedisParseErr::*;
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

    fn after_newline_at(s: &'a str, start: usize) -> RedisParser<'a, &'a str> {
        Ok(s.get(start + "\r\n".len()..).ok_or(Incomplete)?)
    }

    fn parse_number_at(s: &'a str) -> RedisParser<(usize, &'a str)> {
        let len = s
            .chars()
            .position(|c| !c.is_numeric())
            .ok_or(NonNumericInput)?;
        // TODO: Test how this error looks when triggered.  Consider adding new variant
        Ok((s[..len].parse()?, Self::after_newline_at(s, len)?))
    }

    /// Parse a Redis bulk string and return the content of that string and the unparsed remainder.
    ///
    /// All bulk strings have the format `$[LENGTH_OF_ITEM_BODY]\r\n[ITEM_BODY]\r\n`
    fn parse_redis_bulk_string(s: &'a str) -> RedisParser<RedisStructuredText> {
        let (field_len, rest) = Self::parse_number_at(s)?;
        let field_content = rest.get(..field_len).ok_or(Incomplete)?;
        Ok(RedisStructuredText {
            parsed_reply: BulkString(field_content),
            leftover_input: RedisUtf8::from_str(Self::after_newline_at(&rest, field_len)?),
        })
    }

    fn parse_redis_int(s: &'a str) -> RedisParser<RedisStructuredText> {
        let (number, rest) = Self::parse_number_at(s)?;
        let leftover_utf8 = RedisUtf8::from_str(Self::after_newline_at(rest, 0)?);
        Ok(RedisStructuredText {
            parsed_reply: Integer(number),
            leftover_input: leftover_utf8,
        })
    }

    fn parse_redis_array(s: &'a str) -> RedisParser<RedisStructuredText> {
        let (number_of_elements, rest) = Self::parse_number_at(s)?;

        let mut str_left_to_parse = RedisUtf8::from_str(rest);
        let mut inner = Vec::with_capacity(number_of_elements);

        inner.resize(number_of_elements, RedisDataType::Uninitilized);

        for i in (0..number_of_elements).rev() {
            let next_el = Self::new_try_from(str_left_to_parse.valid_utf8)?;
            str_left_to_parse = next_el.leftover_input;
            inner[i] = next_el.parsed_reply;
        }
        Ok(RedisStructuredText {
            parsed_reply: RedisDataType::RedisArray(inner),
            leftover_input: str_left_to_parse,
        })
    }

    fn new_try_from(s: &'a str) -> Result<RedisStructuredText, RedisParseErr> {
        if s.len() < 4 {
            Err(Incomplete)?
        };
        let (first_char, s) = s.split_at(1);
        match first_char {
            ":" => Self::parse_redis_int(s),
            "$" => Self::parse_redis_bulk_string(s),
            "*" => Self::parse_redis_array(s),
            e => Err(InvalidLineStart(format!(
                "Encountered invalid initial character `{}` in line `{}`",
                e, s
            ))),
        }
    }
}

impl<'a> TryFrom<RedisDataType<'a>> for &'a str {
    type Error = RedisParseErr;

    fn try_from(val: RedisDataType<'a>) -> Result<Self, Self::Error> {
        match val {
            RedisDataType::BulkString(inner) => Ok(inner),
            _ => Err(IncorrectRedisType),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_redis_int() -> Result<(), RedisParseErr> {
        let mut buffer = Vec::new();
        let input = "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$45\r\n{\"event\":\"announcement.delete\",\"payload\":\"5\"}\r\n";

        let r_txt = RedisBytes(input.as_bytes()).into_redis_utf8();
        assert_eq!(r_txt.valid_utf8, "*3\r\n$7\r\nmessage\r\n$12\r\ntimeline:308\r\n$45\r\n{\"event\":\"announcement.delete\",\"payload\":\"5\"}\r\n");
        let r_structured = r_txt.try_into_redis_structured_text()?;
        assert_eq!(
            r_structured.parsed_reply,
            RedisArray(vec![
                BulkString(&"{\"event\":\"announcement.delete\",\"payload\":\"5\"}"),
                BulkString(&"timeline:308"),
                BulkString(&"message"),
            ])
        );
        let r_msg = r_structured.try_into_redis_message()?;

        buffer.push(r_msg.leftover_input.as_leftover_bytes());
        assert_eq!(r_msg.timeline_txt, "timeline:308");
        assert_eq!(
            r_msg.event_txt,
            "{\"event\":\"announcement.delete\",\"payload\":\"5\"}"
        );

        Ok(())
    }
}
