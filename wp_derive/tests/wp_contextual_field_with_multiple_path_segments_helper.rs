use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseBar {
    #[WPContext(edit)]
    pub baz: u32,
}
