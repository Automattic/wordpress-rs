use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::{AnyJson, JsonValue, wp_block_editor::WpBlockEditorSettingsParams};
use std::sync::Arc;
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum WpBlockEditorRequest {
    #[get(url = "/settings", params = &WpBlockEditorSettingsParams, output = JsonValue)]
    GetSettings,
    #[get(url = "/settings", params = &WpBlockEditorSettingsParams, output = Arc<AnyJson>)]
    GetRawSettings,
}

impl DerivedRequest for WpBlockEditorRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpBlockEditorV1
    }
}
