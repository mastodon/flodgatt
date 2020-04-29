use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Attachment {
    pub(in super::super) id: String,
    pub(in super::super) r#type: AttachmentType,
    pub(in super::super) url: String,
    pub(in super::super) preview_url: String,
    pub(in super::super) remote_url: Option<String>,
    pub(in super::super) text_url: Option<String>,
    pub(in super::super) meta: Option<serde_json::Value>, // TODO - is this the best type for the API?
    pub(in super::super) description: Option<String>,
    pub(in super::super) blurhash: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum AttachmentType {
    Unknown,
    Image,
    Gifv,
    Video,
    Audio,
}
