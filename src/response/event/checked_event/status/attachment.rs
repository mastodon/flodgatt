use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attachment {
    pub(crate) id: String,
    pub(crate) r#type: AttachmentType,
    pub(crate) url: String,
    pub(crate) preview_url: String,
    pub(crate) remote_url: Option<String>,
    pub(crate) text_url: Option<String>,
    pub(crate) meta: Option<serde_json::Value>, // TODO - is this the best type for the API?
    pub(crate) description: Option<String>,
    pub(crate) blurhash: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttachmentType {
    Unknown,
    Image,
    Gifv,
    Video,
    Audio,
}
