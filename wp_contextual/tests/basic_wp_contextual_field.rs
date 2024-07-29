use std::collections::HashMap;
use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualField]
    pub bar: Option<SparseBar>,
    #[WpContext(edit)]
    #[WpContextualField]
    pub vec_bar: Option<Vec<SparseBar>>,
    #[WpContext(edit)]
    #[WpContextualField]
    pub hash_map_bar: Option<std::collections::HashMap<String, SparseBar>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record, WpContextual)]
pub struct SparseBar {
    #[WpContext(edit)]
    pub baz: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext {
        bar: BarWithEditContext { baz: 0 },
        vec_bar: vec![BarWithEditContext { baz: 1 }],
        hash_map_bar: HashMap::from([("bar_2".to_string(), BarWithEditContext { baz: 2 })]),
    };
    let _ = SparseFooWithEditContext {
        bar: Some(SparseBarWithEditContext { baz: Some(0) }),
        vec_bar: Some(vec![SparseBarWithEditContext { baz: Some(1) }]),
        hash_map_bar: Some(HashMap::from([(
            "bar_2".to_string(),
            SparseBarWithEditContext { baz: Some(2) },
        )])),
    };
    let bar_field = SparseFooFieldWithEditContext::Bar;
    assert_eq!(bar_field.as_field_name(), "bar");
}

uniffi::setup_scaffolding!();
