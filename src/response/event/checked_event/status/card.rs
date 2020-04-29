use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Card {
    pub(super) url: String,
    pub(super) title: String,
    pub(super) description: String,
    pub(super) r#type: CardType,
    pub(super) author_name: Option<String>,
    pub(super) author_url: Option<String>,
    pub(super) provider_name: Option<String>,
    pub(super) provider_url: Option<String>,
    pub(super) html: Option<String>,
    pub(super) width: Option<i64>,
    pub(super) height: Option<i64>,
    pub(super) image: Option<String>,
    pub(super) embed_url: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) enum CardType {
    Link,
    Photo,
    Video,
    Rich,
}
