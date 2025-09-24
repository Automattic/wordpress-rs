use crate::{
    JsonValue, impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::sync::Arc;
use wp_derive::WpDeriveParamsField;

#[derive(Debug, Default, Serialize, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct WpBlockEditorSettingsParams {
    #[uniffi(default = None)]
    pub context: Option<WpBlockEditorSettingsContext>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
pub enum WpBlockEditorSettingsContext {
    #[strum(serialize = "post-editor")]
    PostEditor,
    #[strum(serialize = "widgets-editor")]
    WidgetsEditor,
    #[strum(serialize = "site-editor")]
    SiteEditor,
    #[strum(serialize = "mobile")]
    Mobile,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_to_string!(WpBlockEditorSettingsContext);

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct WpBlockEditorSettings {
    pub payload: Arc<WpBlockEditorSettingsPayload>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Object)]
#[serde(transparent)]
pub struct WpBlockEditorSettingsPayload {
    pub payload: Box<RawValue>,
}

#[uniffi::export]
impl WpBlockEditorSettingsPayload {
    pub fn as_bytes(&self) -> Vec<u8> {
        self.payload.get().as_bytes().to_vec()
    }

    pub fn as_json(&self) -> JsonValue {
        serde_json::from_str(self.payload.get()).unwrap_or(JsonValue::Null)
    }
}

#[cfg(test)]
mod test {
    use crate::JsonValue;
    use rstest::*;

    #[rstest]
    #[case("tests/wp_block_editor/settings/settings-01.json")]
    fn test_parsing_settings(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        assert!(serde_json::from_reader::<_, JsonValue>(file).is_ok());
    }
}
