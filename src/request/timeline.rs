use super::query::Query;
use crate::err::TimelineErr;
use crate::event::Id;

use hashbrown::HashSet;
use lru::LruCache;
use std::convert::TryFrom;
use warp::reject::Rejection;

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub struct Timeline(pub Stream, pub Reach, pub Content);

impl Timeline {
    pub fn empty() -> Self {
        Self(Stream::Unset, Reach::Local, Content::Notification)
    }

    pub fn to_redis_raw_timeline(&self, hashtag: Option<&String>) -> Result<String, TimelineErr> {
        use {Content::*, Reach::*, Stream::*};
        Ok(match self {
            Timeline(Public, Federated, All) => "timeline:public".into(),
            Timeline(Public, Local, All) => "timeline:public:local".into(),
            Timeline(Public, Federated, Media) => "timeline:public:media".into(),
            Timeline(Public, Local, Media) => "timeline:public:local:media".into(),
            // TODO -- would `.push_str` be faster here?
            Timeline(Hashtag(_id), Federated, All) => format!(
                "timeline:hashtag:{}",
                hashtag.ok_or(TimelineErr::MissingHashtag)?
            ),
            Timeline(Hashtag(_id), Local, All) => format!(
                "timeline:hashtag:{}:local",
                hashtag.ok_or(TimelineErr::MissingHashtag)?
            ),
            Timeline(User(id), Federated, All) => format!("timeline:{}", id),
            Timeline(User(id), Federated, Notification) => format!("timeline:{}:notification", id),
            Timeline(List(id), Federated, All) => format!("timeline:list:{}", id),
            Timeline(Direct(id), Federated, All) => format!("timeline:direct:{}", id),
            Timeline(_one, _two, _three) => Err(TimelineErr::InvalidInput)?,
        })
    }

    pub fn from_redis_text(
        timeline: &str,
        cache: &mut LruCache<String, i64>,
    ) -> Result<Self, TimelineErr> {
        // TODO -- can a combinator shorten this?
        let mut id_from_tag = |tag: &str| match cache.get(&tag.to_string()) {
            Some(id) => Ok(*id),
            None => Err(TimelineErr::InvalidInput), // TODO more specific
        };

        use {Content::*, Reach::*, Stream::*};
        Ok(match &timeline.split(':').collect::<Vec<&str>>()[..] {
            ["public"] => Timeline(Public, Federated, All),
            ["public", "local"] => Timeline(Public, Local, All),
            ["public", "media"] => Timeline(Public, Federated, Media),
            ["public", "local", "media"] => Timeline(Public, Local, Media),
            ["hashtag", tag] => Timeline(Hashtag(id_from_tag(tag)?), Federated, All),
            ["hashtag", tag, "local"] => Timeline(Hashtag(id_from_tag(tag)?), Local, All),
            [id] => Timeline(User(id.parse()?), Federated, All),
            [id, "notification"] => Timeline(User(id.parse()?), Federated, Notification),
            ["list", id] => Timeline(List(id.parse()?), Federated, All),
            ["direct", id] => Timeline(Direct(id.parse()?), Federated, All),
            // Other endpoints don't exist:
            [..] => Err(TimelineErr::InvalidInput)?,
        })
    }

    pub fn from_query_and_user(q: &Query, user: &UserData) -> Result<Self, Rejection> {
        use {warp::reject::custom, Content::*, Reach::*, Scope::*, Stream::*};

        Ok(match q.stream.as_ref() {
            "public" => match q.media {
                true => Timeline(Public, Federated, Media),
                false => Timeline(Public, Federated, All),
            },
            "public:local" => match q.media {
                true => Timeline(Public, Local, Media),
                false => Timeline(Public, Local, All),
            },
            "public:media" => Timeline(Public, Federated, Media),
            "public:local:media" => Timeline(Public, Local, Media),

            "hashtag" => Timeline(Hashtag(0), Federated, All),
            "hashtag:local" => Timeline(Hashtag(0), Local, All),
            "user" => match user.scopes.contains(&Statuses) {
                true => Timeline(User(user.id), Federated, All),
                false => Err(custom("Error: Missing access token"))?,
            },
            "user:notification" => match user.scopes.contains(&Statuses) {
                true => Timeline(User(user.id), Federated, Notification),
                false => Err(custom("Error: Missing access token"))?,
            },
            "list" => match user.scopes.contains(&Lists) {
                true => Timeline(List(q.list), Federated, All),
                false => Err(warp::reject::custom("Error: Missing access token"))?,
            },
            "direct" => match user.scopes.contains(&Statuses) {
                true => Timeline(Direct(*user.id), Federated, All),
                false => Err(custom("Error: Missing access token"))?,
            },
            other => {
                log::warn!("Request for nonexistent endpoint: `{}`", other);
                Err(custom("Error: Nonexistent endpoint"))?
            }
        })
    }
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum Stream {
    User(Id),
    // TODO consider whether List, Direct, and Hashtag should all be `id::Id`s
    List(i64),
    Direct(i64),
    Hashtag(i64),
    Public,
    Unset,
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum Reach {
    Local,
    Federated,
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum Content {
    All,
    Media,
    Notification,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Read,
    Statuses,
    Notifications,
    Lists,
}

impl TryFrom<&str> for Scope {
    type Error = TimelineErr;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
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

pub struct UserData {
    pub id: Id,
    pub allowed_langs: HashSet<String>,
    pub scopes: HashSet<Scope>,
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
