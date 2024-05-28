use wp_contextual::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(edit)]
    #[WPContextualOption]
    pub bar: Option<u32>,
}

fn main() {
    let _ = FooWithEditContext { bar: None };
}

uniffi::setup_scaffolding!();
