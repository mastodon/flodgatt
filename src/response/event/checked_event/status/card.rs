use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Card {
    url: String,
    title: String,
    description: String,
    r#type: CardType,
    author_name: Option<String>,
    author_url: Option<String>,
    provider_name: Option<String>,
    provider_url: Option<String>,
    html: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    image: Option<String>,
    embed_url: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum CardType {
    Link,
    Photo,
    Video,
    Rich,
}
