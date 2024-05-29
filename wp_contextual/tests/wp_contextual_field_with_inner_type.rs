use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualField]
    pub bar: Option<Vec<SparseBar>>,
    #[WpContext(edit)]
    #[WpContextualField]
    pub bar_2: Option<std::vec::Vec<SparseBar>>,
    #[WpContext(edit)]
    #[WpContextualField]
    pub bar_3: Option<std::vec::Vec<Vec<SparseBar>>>,
}

#[derive(WpContextual)]
pub struct SparseBar {
    #[WpContext(edit)]
    pub baz: u32,
}

fn main() {
    let _ = FooWithEditContext {
        bar: vec![BarWithEditContext { baz: 0 }],
        bar_2: vec![BarWithEditContext { baz: 0 }],
        bar_3: vec![vec![BarWithEditContext { baz: 0 }]],
    };
}

uniffi::setup_scaffolding!();
