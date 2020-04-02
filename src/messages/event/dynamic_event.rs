use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct DynamicEvent {
    pub event: String,
    pub payload: Value,
    queued_at: Option<i64>,
}

impl DynamicEvent {
    /// Returns `true` if the status is filtered out based on its language
    pub fn language_not(&self, allowed_langs: &HashSet<String>) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;

        if allowed_langs.is_empty() {
            return ALLOW; // listing no allowed_langs results in allowing all languages
        }

        match self.payload["language"].as_str() {
            Some(toot_language) if allowed_langs.contains(toot_language) => ALLOW,
            None => ALLOW, // If toot language is unknown, toot is always allowed
            Some(empty) if empty == &String::new() => ALLOW,
            Some(_toot_language) => REJECT,
        }
    }
    /// Returns `true` if the toot contained in this Event originated from a blocked domain,
    /// is from an account that has blocked the current user, or if the User's list of
    /// blocked/muted users includes a user involved in the toot.
    ///
    /// A user is involved in the toot if they:
    ///  * Are mentioned in this toot
    ///  * Wrote this toot
    ///  * Wrote a toot that this toot is replying to (if any)
    ///  * Wrote the toot that this toot is boosting (if any)
    pub fn involves_any(
        &self,
        blocked_users: &HashSet<i64>,
        blocked_domains: &HashSet<String>,
        blocking_users: &HashSet<i64>,
    ) -> bool {
        const ALLOW: bool = false;
        const REJECT: bool = true;
        let user_id = self.payload["account"]["id"].as_str().expect("TODO");
        let username = self.payload["account"]["acct"].as_str().expect("TODO");

        if !self.calculate_involved_users().is_disjoint(blocked_users) {
            REJECT
        } else if blocking_users.contains(&user_id.parse().expect("TODO")) {
            REJECT
        } else {
            let full_username = &username;
            match full_username.split('@').nth(1) {
                Some(originating_domain) if blocked_domains.contains(originating_domain) => REJECT,
                Some(_) | None => ALLOW, // None means the local instance, which can't be blocked
            }
        }
    }
    fn calculate_involved_users(&self) -> HashSet<i64> {
        let mentions = self.payload["mentions"].as_array().expect("TODO");
        // involved_users = mentioned_users + author + replied-to user + boosted user
        let mut involved_users: HashSet<i64> = mentions
            .iter()
            .map(|mention| mention["id"].as_str().expect("TODO").parse().expect("TODO"))
            .collect();

        // author
        let author_id = self.payload["account"]["id"].as_str().expect("TODO");
        involved_users.insert(author_id.parse::<i64>().expect("TODO"));
        // replied-to user
        let replied_to_user = self.payload["in_reply_to_account_id"].as_str();
        if let Some(user_id) = replied_to_user.clone() {
            involved_users.insert(user_id.parse().expect("TODO"));
        }
        // boosted user

        let id_of_boosted_user = self.payload["reblog"]["account"]["id"]
            .as_str()
            .expect("TODO");
        involved_users.insert(id_of_boosted_user.parse().expect("TODO"));

        involved_users
    }
}
