use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualField]
    pub bar: Option<SparseBar>,
}

#[derive(WpContextual)]
pub struct SparseBar {
    #[WpContext(edit)]
    pub baz: u32,
}

fn main() {
    let _ = FooWithEditContext {
        bar: BarWithEditContext { baz: 0 },
    };
}

uniffi::setup_scaffolding!();
