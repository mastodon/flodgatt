use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

/// A user ID.
///
/// Internally, Mastodon IDs are i64s, but are sent to clients as string because
/// JavaScript numbers don't support i64s.  This newtype serializes to/from a string, but
/// keeps the i64 as the "true" value for internal use.
#[derive(Debug, Clone, PartialEq)]
pub struct Id(pub i64);

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

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        match value.parse() {
            Ok(n) => Ok(Id(n)),
            Err(e) => Err(E::custom(format!("could not parse: {}", e))),
        }
    }
}
