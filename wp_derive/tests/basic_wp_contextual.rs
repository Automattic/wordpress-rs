use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(edit, embed, view)]
    pub bar: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext { bar: 0 };
    let _ = FooWithEmbedContext { bar: 0 };
    let _ = FooWithViewContext { bar: 0 };
}

uniffi::setup_scaffolding!("wp_derive");
