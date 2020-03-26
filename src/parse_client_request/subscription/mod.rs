//! `User` struct and related functionality
// #[cfg(test)]
// mod mock_postgres;
// #[cfg(test)]
// use mock_postgres as postgres;
// #[cfg(not(test))]
pub mod postgres;
pub use self::postgres::PgPool;
use super::query::Query;
use crate::log_fatal;
use std::collections::HashSet;
use warp::reject::Rejection;

/// The User (with data read from Postgres)
#[derive(Clone, Debug, PartialEq)]
pub struct Subscription {
    pub timeline: Timeline,
    pub allowed_langs: HashSet<String>,
    pub blocks: Blocks,
    pub hashtag_name: Option<String>,
}

impl Default for Subscription {
    fn default() -> Self {
        Self {
            timeline: Timeline(Stream::Unset, Reach::Local, Content::Notification),
            allowed_langs: HashSet::new(),
            blocks: Blocks::default(),
            hashtag_name: None,
        }
    }
}

impl Subscription {
    pub fn from_query(q: Query, pool: PgPool, whitelist_mode: bool) -> Result<Self, Rejection> {
        let user = match q.access_token.clone() {
            Some(token) => postgres::select_user(&token, pool.clone())?,
            None if whitelist_mode => Err(warp::reject::custom("Error: Invalid access token"))?,
            None => UserData::public(),
        };
        let timeline = Timeline::from_query_and_user(&q, &user, pool.clone())?;
        let hashtag_name = match timeline {
            Timeline(Stream::Hashtag(_), _, _) => Some(q.hashtag),
            _non_hashtag_timeline => None,
        };

        Ok(Subscription {
            timeline,
            allowed_langs: user.allowed_langs,
            blocks: Blocks {
                blocking_users: postgres::select_blocking_users(user.id, pool.clone()),
                blocked_users: postgres::select_blocked_users(user.id, pool.clone()),
                blocked_domains: postgres::select_blocked_domains(user.id, pool.clone()),
            },
            hashtag_name,
        })
    }
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub struct Timeline(pub Stream, pub Reach, pub Content);

impl Timeline {
    pub fn empty() -> Self {
        use {Content::*, Reach::*, Stream::*};
        Self(Unset, Local, Notification)
    }

    pub fn to_redis_raw_timeline(&self, hashtag: Option<&String>) -> String {
        use {Content::*, Reach::*, Stream::*};
        match self {
            Timeline(Public, Federated, All) => "timeline:public".into(),
            Timeline(Public, Local, All) => "timeline:public:local".into(),
            Timeline(Public, Federated, Media) => "timeline:public:media".into(),
            Timeline(Public, Local, Media) => "timeline:public:local:media".into(),

            Timeline(Hashtag(id), Federated, All) => format!(
                "timeline:hashtag:{}",
                hashtag.unwrap_or_else(|| log_fatal!("Did not supply a name for hashtag #{}", id))
            ),
            Timeline(Hashtag(id), Local, All) => format!(
                "timeline:hashtag:{}:local",
                hashtag.unwrap_or_else(|| log_fatal!("Did not supply a name for hashtag #{}", id))
            ),
            Timeline(User(id), Federated, All) => format!("timeline:{}", id),
            Timeline(User(id), Federated, Notification) => format!("timeline:{}:notification", id),
            Timeline(List(id), Federated, All) => format!("timeline:list:{}", id),
            Timeline(Direct(id), Federated, All) => format!("timeline:direct:{}", id),
            Timeline(one, _two, _three) => {
                log_fatal!("Supposedly impossible timeline reached: {:?}", one)
            }
        }
    }
    pub fn from_redis_raw_timeline(raw_timeline: &str, hashtag: Option<i64>) -> Self {
        use {Content::*, Reach::*, Stream::*};
        match raw_timeline.split(':').collect::<Vec<&str>>()[..] {
            ["public"] => Timeline(Public, Federated, All),
            ["public", "local"] => Timeline(Public, Local, All),
            ["public", "media"] => Timeline(Public, Federated, Media),
            ["public", "local", "media"] => Timeline(Public, Local, Media),

            ["hashtag", _tag] => Timeline(Hashtag(hashtag.unwrap()), Federated, All),
            ["hashtag", _tag, "local"] => Timeline(Hashtag(hashtag.unwrap()), Local, All),
            [id] => Timeline(User(id.parse().unwrap()), Federated, All),
            [id, "notification"] => Timeline(User(id.parse().unwrap()), Federated, Notification),
            ["list", id] => Timeline(List(id.parse().unwrap()), Federated, All),
            ["direct", id] => Timeline(Direct(id.parse().unwrap()), Federated, All),
            // Other endpoints don't exist:
            [..] => log_fatal!("Unexpected channel from Redis: {}", raw_timeline),
        }
    }
    fn from_query_and_user(q: &Query, user: &UserData, pool: PgPool) -> Result<Self, Rejection> {
        use {warp::reject::custom, Content::*, Reach::*, Scope::*, Stream::*};
        let id_from_hashtag = || postgres::select_hashtag_id(&q.hashtag, pool.clone());
        let user_owns_list = || postgres::user_owns_list(user.id, q.list, pool.clone());

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

            "hashtag" => Timeline(Hashtag(id_from_hashtag()?), Federated, All),
            "hashtag:local" => Timeline(Hashtag(id_from_hashtag()?), Local, All),
            "user" => match user.scopes.contains(&Statuses) {
                true => Timeline(User(user.id), Federated, All),
                false => Err(custom("Error: Missing access token"))?,
            },
            "user:notification" => match user.scopes.contains(&Statuses) {
                true => Timeline(User(user.id), Federated, Notification),
                false => Err(custom("Error: Missing access token"))?,
            },
            "list" => match user.scopes.contains(&Lists) && user_owns_list() {
                true => Timeline(List(q.list), Federated, All),
                false => Err(warp::reject::custom("Error: Missing access token"))?,
            },
            "direct" => match user.scopes.contains(&Statuses) {
                true => Timeline(Direct(user.id), Federated, All),
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
    User(i64),
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

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Blocks {
    pub blocked_domains: HashSet<String>,
    pub blocked_users: HashSet<i64>,
    pub blocking_users: HashSet<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserData {
    id: i64,
    allowed_langs: HashSet<String>,
    scopes: HashSet<Scope>,
}

impl UserData {
    fn public() -> Self {
        Self {
            id: -1,
            allowed_langs: HashSet::new(),
            scopes: HashSet::new(),
        }
    }
}
