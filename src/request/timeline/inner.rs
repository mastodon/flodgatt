use super::TimelineErr;
use crate::event::Id;

use hashbrown::HashSet;
use std::convert::TryFrom;

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub(crate) enum Stream {
    User(Id),
    List(i64),
    Direct(i64),
    Hashtag(i64),
    Public,
    Unset,
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub(crate) enum Reach {
    Local,
    Federated,
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub(crate) enum Content {
    All,
    Media,
    Notification,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    Read,
    Statuses,
    Notifications,
    Lists,
}

impl TryFrom<&str> for Scope {
    type Error = TimelineErr;

    fn try_from(s: &str) -> Result<Self, TimelineErr> {
        match s {
            "read" => Ok(Scope::Read),
            "read:statuses" => Ok(Scope::Statuses),
            "read:notifications" => Ok(Scope::Notifications),
            "read:lists" => Ok(Scope::Lists),
            "write" | "follow" => Err(TimelineErr::InvalidInput), // ignore write scopes
            unexpected => {
                log::warn!("Ignoring unknown scope `{}`", unexpected);
                Err(TimelineErr::InvalidInput)
            }
        }
    }
}

pub(crate) struct UserData {
    pub(crate) id: Id,
    pub(crate) allowed_langs: HashSet<String>,
    pub(crate) scopes: HashSet<Scope>,
}

impl UserData {
    pub fn public() -> Self {
        Self {
            id: Id(-1),
            allowed_langs: HashSet::new(),
            scopes: HashSet::new(),
        }
    }
}
