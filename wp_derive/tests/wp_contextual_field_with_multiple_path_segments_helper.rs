// Helper mod to be able to test multiple path segments in
// wp_contextual_field_with_multiple_path_segments

use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseBar {
    #[WPContext(edit)]
    pub baz: u32,
}
