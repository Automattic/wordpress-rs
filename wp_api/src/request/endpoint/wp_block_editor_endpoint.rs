use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{JsonValue, wp_block_editor::WpBlockEditorSettingsParams};
use crate::wp_block_editor::RawSettings;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum WpBlockEditorRequest {
    #[get(url = "/settings", params = &WpBlockEditorSettingsParams, output = JsonValue)]
    GetSettings,
    #[get(url = "/settings", params = &WpBlockEditorSettingsParams, output = RawSettings)]
    GetRawSettings,
}

impl DerivedRequest for WpBlockEditorRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpBlockEditorV1
    }
}
