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

#[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record, WpContextual)]
pub struct SparseBar {
    #[WpContext(edit)]
    pub baz: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext {
        bar: vec![BarWithEditContext { baz: 0 }],
        bar_2: vec![BarWithEditContext { baz: 0 }],
        bar_3: vec![vec![BarWithEditContext { baz: 0 }]],
    };
    let _ = SparseFooWithEditContext {
        bar: Some(vec![SparseBarWithEditContext { baz: Some(0) }]),
        bar_2: Some(vec![SparseBarWithEditContext { baz: Some(0) }]),
        bar_3: Some(vec![vec![SparseBarWithEditContext { baz: Some(0) }]]),
    };
}

uniffi::setup_scaffolding!();
