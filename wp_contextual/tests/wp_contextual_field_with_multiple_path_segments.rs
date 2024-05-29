use wp_contextual::WpContextual;

mod wp_contextual_field_with_multiple_segments_helper;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualField]
    pub bar: Option<wp_contextual_field_with_multiple_segments_helper::SparseBar>,
}

fn main() {
    let _ = FooWithEditContext {
        bar: wp_contextual_field_with_multiple_segments_helper::BarWithEditContext { baz: 0 },
    };
}

uniffi::setup_scaffolding!();
