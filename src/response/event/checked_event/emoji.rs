use serde::{Deserialize, Serialize};

#[serde(deny_unknown_fields)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(super) struct Emoji {
    shortcode: String,
    url: String,
    static_url: String,
    visible_in_picker: bool,
    category: Option<String>,
}
