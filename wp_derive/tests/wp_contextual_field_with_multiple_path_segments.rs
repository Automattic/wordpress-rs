use wp_derive::WPContextual;

mod wp_contextual_field_with_multiple_segments_helper;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(edit)]
    #[WPContextualField]
    pub bar: Option<wp_contextual_field_with_multiple_segments_helper::SparseBar>,
}

fn main() {
    let _ = FooWithEditContext {
        bar: wp_contextual_field_with_multiple_segments_helper::BarWithEditContext { baz: 0 },
    };
}

uniffi::setup_scaffolding!("wp_derive");
