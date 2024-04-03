use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(edit)]
    #[WPContextualField]
    pub bar: Option<SparseBar>,
}

#[derive(WPContextual)]
pub struct SparseBar {
    #[WPContext(edit)]
    pub baz: u32,
}

fn main() {
    let _ = FooWithEditContext {
        bar: BarWithEditContext { baz: 0 },
    };
}

uniffi::setup_scaffolding!();
