use serde::Deserialize;

use crate::{AnyJson, uniffi_serde::UniffiSerializationError};

#[derive(Debug, Deserialize, uniffi::Record)]
pub struct WpComCommentExtension {
    #[serde(rename = "extended_post")]
    pub post: Option<WpComCommentExtensionPostInfo>,
    #[serde(rename = "extended_i_replied")]
    pub i_replied: bool,
    #[serde(rename = "extended_like_count")]
    pub like_count: u32,
    #[serde(rename = "extended_i_like")]
    pub i_like: bool,
}

#[derive(Debug, Deserialize, uniffi::Record)]
pub struct WpComCommentExtensionPostInfo {
    pub id: u64,
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub link: String,
}

#[uniffi::export(with_foreign)]
pub trait WpComCommentExtensionProvider: Send + Sync {
    fn parse_extension(&self) -> Result<WpComCommentExtension, UniffiSerializationError>;
}

#[uniffi::export]
impl WpComCommentExtensionProvider for AnyJson {
    fn parse_extension(&self) -> Result<WpComCommentExtension, UniffiSerializationError> {
        serde_json::to_string(&self.raw)
            .and_then(|json| serde_json::from_str(&json))
            .map_err(Into::into)
    }
}
