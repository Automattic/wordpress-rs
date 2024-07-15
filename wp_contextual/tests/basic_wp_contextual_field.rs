use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualField]
    pub bar: Option<SparseBar>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record, WpContextual)]
pub struct SparseBar {
    #[WpContext(edit)]
    pub baz: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext {
        bar: BarWithEditContext { baz: 0 },
    };
    let _ = SparseFooWithEditContext {
        bar: Some(SparseBarWithEditContext { baz: Some(0) }),
    };
    let bar_field = SparseFooFieldWithEditContext::Bar;
    assert_eq!(bar_field.as_field_name(), "bar");
}

uniffi::setup_scaffolding!();
