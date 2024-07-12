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
    let bar_field = SparseFooFieldWithEditContext::Bar;
    let bar_2_field = SparseFooFieldWithEditContext::Bar2;
    let bar_3_field = SparseFooFieldWithEditContext::Bar3;
    assert_eq!(bar_field.as_field_name(), "bar");
    assert_eq!(bar_2_field.as_field_name(), "bar_2");
    assert_eq!(bar_3_field.as_field_name(), "bar_3");
    let baz_field = SparseBarFieldWithEditContext::Baz;
    assert_eq!(baz_field.as_field_name(), "baz");
}

uniffi::setup_scaffolding!();
