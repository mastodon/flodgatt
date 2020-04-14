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

mod err;
pub use err::RedisParseErr;

use self::RedisParseOutput::*;

use std::{
    convert::{TryFrom, TryInto},
    str,
};

#[derive(Debug, Clone, PartialEq)]
pub enum RedisParseOutput<'a> {
    Msg(RedisMsg<'a>),
    NonMsg(&'a str),
}

// TODO -- should this impl Iterator?

#[derive(Debug, Clone, PartialEq)]
pub struct RedisMsg<'a> {
    pub timeline_txt: &'a str,
    pub event_txt: &'a str,
    pub leftover_input: &'a str,
}

impl<'a> TryFrom<&'a str> for RedisParseOutput<'a> {
    type Error = RedisParseErr;
    fn try_from(utf8: &'a str) -> Result<RedisParseOutput<'a>, Self::Error> {
        let (structured_txt, leftover_utf8) = utf8_to_redis_data(utf8)?;
        let structured_txt = RedisStructuredText {
            structured_txt,
            leftover_input: leftover_utf8,
        };
        Ok(structured_txt.try_into()?)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct RedisStructuredText<'a> {
    structured_txt: RedisData<'a>,
    leftover_input: &'a str,
}
#[derive(Debug, Clone, PartialEq)]
enum RedisData<'a> {
    RedisArray(Vec<RedisData<'a>>),
    BulkString(&'a str),
    Integer(usize),
    Uninitilized,
}

use RedisData::*;
use RedisParseErr::*;
type RedisParser<'a, Item> = Result<Item, RedisParseErr>;
fn utf8_to_redis_data<'a>(s: &'a str) -> Result<(RedisData, &'a str), RedisParseErr> {
    if s.len() < 4 {
        Err(Incomplete)?
    };
    let (first_char, s) = s.split_at(1);
    match first_char {
        ":" => parse_redis_int(s),
        "$" => parse_redis_bulk_string(s),
        "*" => parse_redis_array(s),
        e => Err(InvalidLineStart(e.to_string())),
    }
}

fn after_newline_at(s: &str, start: usize) -> RedisParser<&str> {
    let s = s.get(start..).ok_or(Incomplete)?;
    if !s.starts_with("\r\n") {
        return Err(RedisParseErr::InvalidLineEnd);
    }
    Ok(s.get("\r\n".len()..).ok_or(Incomplete)?)
}

fn parse_number_at<'a>(s: &'a str) -> RedisParser<(usize, &'a str)> {
    let len = s.chars().position(|c| !c.is_numeric()).ok_or(Incomplete)?;
    Ok((s[..len].parse()?, after_newline_at(s, len)?))
}

/// Parse a Redis bulk string and return the content of that string and the unparsed remainder.
///
/// All bulk strings have the format `$[LENGTH_OF_ITEM_BODY]\r\n[ITEM_BODY]\r\n`
fn parse_redis_bulk_string<'a>(s: &'a str) -> RedisParser<(RedisData, &'a str)> {
    let (len, rest) = parse_number_at(s)?;
    let content = rest.get(..len).ok_or(Incomplete)?;
    Ok((BulkString(content), after_newline_at(&rest, len)?))
}

fn parse_redis_int<'a>(s: &'a str) -> RedisParser<(RedisData, &'a str)> {
    let (number, rest) = parse_number_at(s)?;
    Ok((Integer(number), rest))
}

fn parse_redis_array<'a>(s: &'a str) -> RedisParser<(RedisData, &'a str)> {
    let (number_of_elements, mut rest) = parse_number_at(s)?;

    let mut inner = Vec::with_capacity(number_of_elements);
    inner.resize(number_of_elements, RedisData::Uninitilized);

    for i in (0..number_of_elements).rev() {
        let (next_el, new_rest) = utf8_to_redis_data(rest)?;
        rest = new_rest;
        inner[i] = next_el;
    }
    Ok((RedisData::RedisArray(inner), rest))
}

impl<'a> TryFrom<RedisData<'a>> for &'a str {
    type Error = RedisParseErr;

    fn try_from(val: RedisData<'a>) -> Result<Self, Self::Error> {
        match val {
            RedisData::BulkString(inner) => Ok(inner),
            _ => Err(IncorrectRedisType),
        }
    }
}

impl<'a> TryFrom<RedisStructuredText<'a>> for RedisParseOutput<'a> {
    type Error = RedisParseErr;

    fn try_from(input: RedisStructuredText<'a>) -> Result<RedisParseOutput<'a>, Self::Error> {
        if let RedisData::RedisArray(mut redis_strings) = input.structured_txt {
            let command = redis_strings.pop().ok_or(MissingField)?.try_into()?;
            match command {
                // subscription statuses look like:
                // $14\r\ntimeline:local\r\n
                // :47\r\n
                "subscribe" | "unsubscribe" => Ok(NonMsg(input.leftover_input)),
                // Messages look like;
                // $10\r\ntimeline:4\r\n
                // $1386\r\n{\"event\":\"update\",\"payload\"...\"queued_at\":1569623342825}\r\n
                "message" => Ok(Msg(RedisMsg {
                    timeline_txt: redis_strings.pop().ok_or(MissingField)?.try_into()?,
                    event_txt: redis_strings.pop().ok_or(MissingField)?.try_into()?,
                    leftover_input: input.leftover_input,
                })),
                _cmd => Err(Incomplete),
            }
        } else {
            Err(IncorrectRedisType)
        }
    }
}

#[cfg(test)]
mod test;
