use crate::log_fatal;
use serde_json::Value;
use std::{collections::HashSet, string::String};
use strum_macros::Display;

#[derive(Debug, Display, Clone)]
pub enum Message {
    Update(Status),
    Conversation(Value),
    Notification(Value),
    Delete(String),
    FiltersChanged,
    Announcement(AnnouncementType),
    UnknownEvent(String, Value),
}

#[derive(Debug, Clone)]
pub struct Status(Value);

#[derive(Debug, Clone)]
pub enum AnnouncementType {
    New(Value),
    Delete(String),
    Reaction(Value),
}

impl Message {
    pub fn from_json(json: Value) -> Self {
        use AnnouncementType::*;
        let event = json["event"]
            .as_str()
            .unwrap_or_else(|| log_fatal!("Could not process `event` in {:?}", json));
        match event {
            "update" => Self::Update(Status(json["payload"].clone())),
            "conversation" => Self::Conversation(json["payload"].clone()),
            "notification" => Self::Notification(json["payload"].clone()),
            "delete" => Self::Delete(
                json["payload"]
                    .as_str()
                    .unwrap_or_else(|| log_fatal!("Could not process `payload` in {:?}", json))
                    .to_string(),
            ),
            "filters_changed" => Self::FiltersChanged,
            "announcement" => Self::Announcement(New(json["payload"].clone())),
            "announcement.reaction" => Self::Announcement(Reaction(json["payload"].clone())),
            "announcement.delete" => Self::Announcement(Delete(
                json["payload"]
                    .as_str()
                    .unwrap_or_else(|| log_fatal!("Could not process `payload` in {:?}", json))
                    .to_string(),
            )),
            other => {
                log::warn!("Received unexpected `event` from Redis: {}", other);
                Self::UnknownEvent(event.to_string(), json["payload"].clone())
            }
        }
    }
    pub fn event(&self) -> String {
        use AnnouncementType::*;
        match self {
            Self::Update(_) => "update",
            Self::Conversation(_) => "conversation",
            Self::Notification(_) => "notification",
            Self::Announcement(New(_)) => "announcement",
            Self::Announcement(Reaction(_)) => "announcement.reaction",
            Self::UnknownEvent(event, _) => &event,
            Self::Delete(_) => "delete",
            Self::Announcement(Delete(_)) => "announcement.delete",
            Self::FiltersChanged => "filters_changed",
        }
        .to_string()
    }
    pub fn payload(&self) -> String {
        use AnnouncementType::*;
        match self {
            Self::Update(status) => status.0.to_string(),
            Self::Conversation(value)
            | Self::Notification(value)
            | Self::Announcement(New(value))
            | Self::Announcement(Reaction(value))
            | Self::UnknownEvent(_, value) => value.to_string(),
            Self::Delete(id) | Self::Announcement(Delete(id)) => id.clone(),
            Self::FiltersChanged => "".to_string(),
        }
    }
}

impl Status {
    /// Returns `true` if the status is filtered out based on its language
    pub fn language_not_allowed(&self, allowed_langs: &HashSet<String>) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;

        let reject_and_maybe_log = |toot_language| {
            log::info!("Filtering out toot from `{}`", &self.0["account"]["acct"]);
            log::info!("Toot language: `{}`", toot_language);
            log::info!("Recipient's allowed languages: `{:?}`", allowed_langs);
            REJECT
        };
        if allowed_langs.is_empty() {
            return ALLOW; // listing no allowed_langs results in allowing all languages
        }
        match self.0["language"].as_str() {
            Some(toot_language) if allowed_langs.contains(toot_language) => ALLOW,
            Some(toot_language) => reject_and_maybe_log(toot_language),
            None => ALLOW, // If toot language is null, toot is always allowed
        }
    }

    /// Returns `true` if this toot originated from a domain the User has blocked.
    pub fn from_blocked_domain(&self, blocked_domains: &HashSet<String>) -> bool {
        let full_username = self.0["account"]["acct"]
            .as_str()
            .unwrap_or_else(|| log_fatal!("Could not process `account.acct` in {:?}", self.0));

        match full_username.split('@').nth(1) {
            Some(originating_domain) => blocked_domains.contains(originating_domain),
            None => false, // None means the user is on the local instance, which can't be blocked
        }
    }
    /// Returns `true` if the Status is from an account that has blocked the current user.
    pub fn from_blocking_user(&self, blocking_users: &HashSet<i64>) -> bool {
        let toot = self.0.clone();
        const ALLOW: bool = false;
        const REJECT: bool = true;

        let author = toot["account"]["id"]
            .str_to_i64()
            .unwrap_or_else(|_| log_fatal!("Could not process `account.id` in {:?}", toot));

        if blocking_users.contains(&author) {
            REJECT
        } else {
            ALLOW
        }
    }

    /// Returns `true` if the User's list of blocked and muted users includes a user
    /// involved in this toot.
    ///
    /// A user is involved if they:
    ///  * Wrote this toot
    ///  * Are mentioned in this toot
    ///  * Wrote a toot that this toot is replying to (if any)
    ///  * Wrote the toot that this toot is boosting (if any)
    pub fn involves_blocked_user(&self, blocked_users: &HashSet<i64>) -> bool {
        let toot = self.0.clone();
        const ALLOW: bool = false;
        const REJECT: bool = true;

        let author_user = match toot["account"]["id"].str_to_i64() {
            Ok(user_id) => vec![user_id].into_iter(),
            Err(_) => log_fatal!("Could not process `account.id` in {:?}", toot),
        };

        let mentioned_users = (match &toot["mentions"] {
            Value::Array(inner) => inner,
            _ => log_fatal!("Could not process `mentions` in {:?}", toot),
        })
        .into_iter()
        .map(|mention| match mention["id"].str_to_i64() {
            Ok(user_id) => user_id,
            Err(_) => log_fatal!("Could not process `id` field of mention in {:?}", toot),
        });

        let replied_to_user = match toot["in_reply_to_account_id"].str_to_i64() {
            Ok(user_id) => vec![user_id].into_iter(),
            Err(_) => vec![].into_iter(), // no error; just no replied_to_user
        };

        let boosted_user = match toot["reblog"].as_object() {
            Some(boosted_user) => match boosted_user["account"]["id"].str_to_i64() {
                Ok(user_id) => vec![user_id].into_iter(),
                Err(_) => log_fatal!("Could not process `reblog.account.id` in {:?}", toot),
            },
            None => vec![].into_iter(), // no error; just no boosted_user
        };

        let involved_users = author_user
            .chain(mentioned_users)
            .chain(replied_to_user)
            .chain(boosted_user)
            .collect::<HashSet<i64>>();

        if involved_users.is_disjoint(blocked_users) {
            ALLOW
        } else {
            REJECT
        }
    }
}

trait ConvertValue {
    fn str_to_i64(&self) -> Result<i64, Box<dyn std::error::Error>>;
}

impl ConvertValue for Value {
    fn str_to_i64(&self) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(self.as_str().ok_or("none_err")?.parse()?)
    }
}
