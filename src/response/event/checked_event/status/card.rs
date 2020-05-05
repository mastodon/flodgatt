use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Card {
    pub(crate) url: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) r#type: CardType,
    pub(crate) author_name: Option<String>,
    pub(crate) author_url: Option<String>,
    pub(crate) provider_name: Option<String>,
    pub(crate) provider_url: Option<String>,
    pub(crate) html: Option<String>,
    pub(crate) width: Option<i64>,
    pub(crate) height: Option<i64>,
    pub(crate) image: Option<String>,
    pub(crate) embed_url: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) enum CardType {
    Link,
    Photo,
    Video,
    Rich,
}
