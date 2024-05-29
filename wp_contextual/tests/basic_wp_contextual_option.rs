use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub bar: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext { bar: None };
}

uniffi::setup_scaffolding!();
