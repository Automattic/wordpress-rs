use crate::impl_as_query_value_from_to_string;
use crate::url_query::AppendUrlQueryPairs;
use crate::url_query::FromUrlQueryPairs;
use crate::url_query::QueryPairs;
use crate::url_query::QueryPairsExtension;
use crate::url_query::UrlQueryPairsMap;
use serde::{Deserialize, Serialize};
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
