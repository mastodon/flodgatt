use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Attachment {
    pub(super) id: String,
    pub(super) r#type: AttachmentType,
    pub(super) url: String,
    pub(super) preview_url: String,
    pub(super) remote_url: Option<String>,
    pub(super) text_url: Option<String>,
    pub(super) meta: Option<serde_json::Value>,
    pub(super) description: Option<String>,
    pub(super) blurhash: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) enum AttachmentType {
    Unknown,
    Image,
    Gifv,
    Video,
    Audio,
}
