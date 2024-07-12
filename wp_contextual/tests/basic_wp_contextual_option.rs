use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub bar: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext { bar: None };
    let _ = SparseFooWithEditContext { bar: Some(0) };
    let bar_field = SparseFooFieldWithEditContext::Bar;
    assert_eq!(bar_field.as_field_name(), "bar");
}

uniffi::setup_scaffolding!();
