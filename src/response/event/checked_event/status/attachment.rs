use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct Attachment {
    id: String,
    r#type: AttachmentType,
    url: String,
    preview_url: String,
    remote_url: Option<String>,
    text_url: Option<String>,
    meta: Option<serde_json::Value>,
    description: Option<String>,
    blurhash: Option<String>,
}

#[serde(rename_all = "lowercase", deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum AttachmentType {
    Unknown,
    Image,
    Gifv,
    Video,
    Audio,
}
