use wp_derive::WPContextual;

// This test is validating that we are able to handle `#[WPContextualField]`s if its type
// has multiple path segments. That's why we use a helper mod and use fully qualified paths
// rather than the importing the mod.
mod wp_contextual_field_with_multiple_path_segments_helper;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(edit)]
    #[WPContextualField]
    pub bar: Option<wp_contextual_field_with_multiple_path_segments_helper::SparseBar>,
}

fn main() {
    let _ = FooWithEditContext {
        bar: wp_contextual_field_with_multiple_path_segments_helper::BarWithEditContext { baz: 0 },
    };
}

uniffi::setup_scaffolding!("wp_derive");
