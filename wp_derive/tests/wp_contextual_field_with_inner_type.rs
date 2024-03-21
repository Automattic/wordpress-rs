use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(edit)]
    #[WPContextualField]
    pub bar: Option<Vec<SparseBar>>,
    #[WPContext(edit)]
    #[WPContextualField]
    pub bar_2: Option<std::vec::Vec<SparseBar>>,
    #[WPContext(edit)]
    #[WPContextualField]
    pub bar_3: Option<std::vec::Vec<Vec<SparseBar>>>,
}

#[derive(WPContextual)]
pub struct SparseBar {
    #[WPContext(edit)]
    pub baz: u32,
}

fn main() {
    let _ = FooWithEditContext {
        bar: vec![BarWithEditContext { baz: 0 }],
        bar_2: vec![BarWithEditContext { baz: 0 }],
        bar_3: vec![vec![BarWithEditContext { baz: 0 }]],
    };
}

uniffi::setup_scaffolding!("wp_derive");
