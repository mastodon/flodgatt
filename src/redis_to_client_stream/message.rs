use crate::parse_client_request::user::Blocks;
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
}

#[derive(Debug, Clone)]
pub struct Status(pub Value);

impl Message {
    pub fn from_json(json: Value) -> Self {
        match json["event"].as_str().unwrap() {
            "update" => Self::Update(Status(json["payload"].clone())),
            "conversation" => Self::Conversation(json["payload"].clone()),
            "notification" => Self::Notification(json["payload"].clone()),
            "delete" => Self::Delete(json["payload"].to_string()),
            "filters_changed" => Self::FiltersChanged,
            _ => unreachable!(),
        }
    }
    pub fn event(&self) -> String {
        format!("{}", self)
    }
    pub fn payload(&self) -> String {
        match self {
            Self::Delete(id) => id.clone(),
            Self::Update(status) => status.0.to_string(),
            Self::Conversation(value) | Self::Notification(value) => value.to_string(),
            Self::FiltersChanged => "".to_string(),
        }
    }
}

impl Status {
    pub fn get_originating_domain(&self) -> HashSet<String> {
        let api = "originating  Invariant Violation: JSON value does not conform to Mastodon API";
        let mut originating_domain = HashSet::new();
        // TODO: make this log an error instead of panicking.
        originating_domain.insert(
            self.0["account"]["acct"]
                .as_str()
                .expect(&api)
                .split('@')
                .nth(1)
                .expect(&api)
                .to_string(),
        );
        originating_domain
    }

    pub fn get_involved_users(&self) -> HashSet<i64> {
        let mut involved_users: HashSet<i64> = HashSet::new();
        let msg = self.0.clone();

        let api = "Invariant Violation: JSON value does not conform to Mastodon API";
        involved_users.insert(msg["account"]["id"].str_to_i64().expect(&api));
        if let Some(mentions) = msg["mentions"].as_array() {
            for mention in mentions {
                involved_users.insert(mention["id"].str_to_i64().expect(&api));
            }
        }
        if let Some(replied_to_account) = msg["in_reply_to_account_id"].as_str() {
            involved_users.insert(replied_to_account.parse().expect(&api));
        }

        if let Some(reblog) = msg["reblog"].as_object() {
            involved_users.insert(reblog["account"]["id"].str_to_i64().expect(&api));
        }
        involved_users
    }

    pub fn is_filtered_out(&self, permitted_langs: &HashSet<String>) -> bool {
        // TODO add logging
        let toot_language = self.0["language"]
            .as_str()
            .expect("Valid language")
            .to_string();
        !{ permitted_langs.is_empty() || permitted_langs.contains(&toot_language) }
    }

    /// Returns `true` if the status is blocked by _either_ domain blocks or _user_ blocks
    pub fn is_blocked(&self, b: &Blocks) -> bool {
        // TODO add logging
        !{
            b.domain_blocks.is_disjoint(&self.get_originating_domain())
                && b.user_blocks.is_disjoint(&self.get_involved_users())
        }
    }
}

trait ConvertValue {
    fn str_to_i64(&self) -> Result<i64, Box<dyn std::error::Error>>;
}

impl ConvertValue for Value {
    fn str_to_i64(&self) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(self
            .as_str()
            .ok_or(format!("{} is not a string", &self))?
            .parse()
            .map_err(|_| "Could not parse str")?)
    }
}
