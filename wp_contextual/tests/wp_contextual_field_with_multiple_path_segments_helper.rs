// Helper mod to be able to test multiple path segments in
// wp_contextual_field_with_multiple_path_segments

use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseBar {
    #[WpContext(edit)]
    pub baz: u32,
}
