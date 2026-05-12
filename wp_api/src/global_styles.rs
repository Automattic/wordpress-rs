use crate::{JsonValue, wp_content_i64_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_contextual::WpContextual;

wp_content_i64_id!(GlobalStylesId);

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseGlobalStyles {
    #[WpContext(edit, embed, view)]
    pub id: Option<GlobalStylesId>,
    #[WpContext(edit, embed, view)]
    pub title: Option<SparseGlobalStylesTitleWrapper>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub settings: Option<HashMap<String, JsonValue>>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub styles: Option<HashMap<String, JsonValue>>,
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SparseGlobalStylesTitleWrapper {
    Object(SparseGlobalStylesTitle),
    String(String),
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseGlobalStylesTitle {
    pub raw: Option<String>,
    pub rendered: Option<String>,
}
