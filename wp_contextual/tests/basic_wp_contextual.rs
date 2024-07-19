use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit, embed, view)]
    pub bar: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext { bar: 0 };
    let _ = FooWithEmbedContext { bar: 0 };
    let _ = FooWithViewContext { bar: 0 };
    let _ = SparseFooWithEditContext { bar: None };
    let _ = SparseFooWithEmbedContext { bar: Some(0) };
    let _ = SparseFooWithViewContext { bar: Some(0) };
    let bar_field_edit_context = SparseFooFieldWithEditContext::Bar;
    assert_eq!(bar_field_edit_context.as_field_name(), "bar");
    let bar_field_embed_context = SparseFooFieldWithEmbedContext::Bar;
    assert_eq!(bar_field_embed_context.as_field_name(), "bar");
    let bar_field_view_context = SparseFooFieldWithViewContext::Bar;
    assert_eq!(bar_field_view_context.as_field_name(), "bar");
}

uniffi::setup_scaffolding!();
