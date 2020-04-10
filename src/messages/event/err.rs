use std::{fmt, num::ParseIntError};

#[derive(Debug)]
pub enum EventErr {
    SerdeParse(serde_json::Error),
    NonNumId(ParseIntError),
    DynParse,
}

impl std::error::Error for EventErr {}

impl fmt::Display for EventErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use EventErr::*;
        match self {
            SerdeParse(inner) => write!(f, "{}", inner),
            NonNumId(inner) => write!(f, "ID could not be parsed: {}", inner),
            DynParse => write!(f, "Could not find a required field in input JSON"),
        }?;
        Ok(())
    }
}

impl From<ParseIntError> for EventErr {
    fn from(error: ParseIntError) -> Self {
        Self::NonNumId(error)
    }
}
impl From<serde_json::Error> for EventErr {
    fn from(error: serde_json::Error) -> Self {
        Self::SerdeParse(error)
    }
}
