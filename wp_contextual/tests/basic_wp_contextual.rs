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
}

uniffi::setup_scaffolding!();
