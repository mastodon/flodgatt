//! `User` struct and related functionality
// #[cfg(test)]
// mod mock_postgres;
// #[cfg(test)]
// use mock_postgres as postgres;
// #[cfg(not(test))]

use super::postgres::PgPool;
use super::query::Query;
use super::{Content, Reach, Stream, Timeline};
use crate::Id;

use hashbrown::HashSet;

use warp::reject::Rejection;

#[derive(Clone, Debug, PartialEq)]
pub struct Subscription {
    pub timeline: Timeline,
    pub allowed_langs: HashSet<String>,
    /// [Blocks](./request/struct.Blocks.html)
    pub blocks: Blocks,
    pub hashtag_name: Option<String>,
    pub access_token: Option<String>,
}

/// Blocked and muted users and domains
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Blocks {
    pub blocked_domains: HashSet<String>,
    pub blocked_users: HashSet<Id>,
    pub blocking_users: HashSet<Id>,
}

impl Default for Subscription {
    fn default() -> Self {
        Self {
            timeline: Timeline(Stream::Unset, Reach::Local, Content::Notification),
            allowed_langs: HashSet::new(),
            blocks: Blocks::default(),
            hashtag_name: None,
            access_token: None,
        }
    }
}

impl Subscription {
    pub(super) fn query_postgres(q: Query, pool: PgPool) -> Result<Self, Rejection> {
        let user = pool.clone().select_user(&q.access_token)?;
        let timeline = {
            let tl = Timeline::from_query_and_user(&q, &user)?;
            let pool = pool.clone();
            use Stream::*;
            match tl {
                Timeline(Hashtag(_), reach, stream) => {
                    let tag = pool.select_hashtag_id(&q.hashtag)?;
                    Timeline(Hashtag(tag), reach, stream)
                }
                Timeline(List(list_id), _, _) if !pool.user_owns_list(user.id, list_id)? => {
                    Err(warp::reject::custom("Error: Missing access token"))?
                }
                other_tl => other_tl,
            }
        };

        let hashtag_name = match timeline {
            Timeline(Stream::Hashtag(_), _, _) => Some(q.hashtag),
            _non_hashtag_timeline => None,
        };

        Ok(Subscription {
            timeline,
            allowed_langs: user.allowed_langs,
            blocks: Blocks {
                blocking_users: pool.clone().select_blocking_users(user.id)?,
                blocked_users: pool.clone().select_blocked_users(user.id)?,
                blocked_domains: pool.select_blocked_domains(user.id)?,
            },
            hashtag_name,
            access_token: q.access_token,
        })
    }
}
