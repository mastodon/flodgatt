use super::super::EventErr;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::Value;
use std::{convert::TryFrom, fmt, num::ParseIntError, str::FromStr};

/// A user ID.
///
/// Internally, Mastodon IDs are i64s, but are sent to clients as string because
/// JavaScript numbers don't support i64s.  This newtype serializes to/from a string, but
/// keeps the i64 as the "true" value for internal use.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Id(pub i64);

impl TryFrom<&Value> for Id {
    type Error = EventErr;

    fn try_from(v: &Value) -> Result<Self, Self::Error> {
        Ok(v.as_str().ok_or(EventErr::DynParse)?.parse()?)
    }
}

impl std::ops::Deref for Id {
    type Target = i64;
    fn deref(&self) -> &i64 {
        &self.0
    }
}
impl FromStr for Id {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Id, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(IdVisitor)
    }
}

struct IdVisitor;
impl<'de> Visitor<'de> for IdVisitor {
    type Value = Id;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string that can be parsed into an i64")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        match value.parse() {
            Ok(n) => Ok(Id(n)),
            Err(e) => Err(E::custom(format!("could not parse: {}", e))),
        }
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        match value.parse() {
            Ok(n) => Ok(Id(n)),
            Err(e) => Err(E::custom(format!("could not parse: {}", e))),
        }
    }
}
