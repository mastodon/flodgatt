use std::{fmt, num::ParseIntError};

#[derive(Debug)]
pub enum Event {
    SerdeParse(serde_json::Error),
    NonNumId(ParseIntError),
    DynParse,
}

impl std::error::Error for Event {}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use Event::*;
        match self {
            SerdeParse(inner) => write!(f, "{}", inner),
            NonNumId(inner) => write!(f, "ID could not be parsed: {}", inner),
            DynParse => write!(f, "Could not find a required field in input JSON"),
        }?;
        Ok(())
    }
}

impl From<ParseIntError> for Event {
    fn from(error: ParseIntError) -> Self {
        Self::NonNumId(error)
    }
}
impl From<serde_json::Error> for Event {
    fn from(error: serde_json::Error) -> Self {
        Self::SerdeParse(error)
    }
}
