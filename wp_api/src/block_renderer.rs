use crate::{JsonValue, posts::PostId, wp_content_string_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_contextual::WpContextual;

wp_content_string_id!(BlockName);

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct BlockRendererPostParams {
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, JsonValue>>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<PostId>,
}

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseBlockRenderer {
    #[WpContext(edit)]
    pub rendered: Option<String>,
}
