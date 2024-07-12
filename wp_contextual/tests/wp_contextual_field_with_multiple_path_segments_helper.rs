// Helper mod to be able to test multiple path segments in
// wp_contextual_field_with_multiple_path_segments

use wp_contextual::WpContextual;

#[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record, WpContextual)]
pub struct SparseBar {
    #[WpContext(edit)]
    pub baz: Option<u32>,
}
