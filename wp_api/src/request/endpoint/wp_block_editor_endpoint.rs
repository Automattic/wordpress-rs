use super::{AsNamespace, DerivedRequest, WpNamespace};
use crate::wp_block_editor::{WpBlockEditorSettings, WpBlockEditorSettingsParams};
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum WpBlockEditorRequest {
    #[get(url = "/settings", params = &WpBlockEditorSettingsParams, output = WpBlockEditorSettings)]
    RetrieveSettings,
}

impl DerivedRequest for WpBlockEditorRequest {
    fn namespace(&self) -> impl AsNamespace {
        WpNamespace::WpBlockEditorV1
    }
}
