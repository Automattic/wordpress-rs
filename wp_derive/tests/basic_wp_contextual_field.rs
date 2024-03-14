use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext("edit")]
    #[WPContextualField]
    pub foo: Option<SparseBaz>,
}

#[derive(WPContextual)]
pub struct SparseBaz {
    #[WPContext("edit")]
    pub qux: u32,
}

fn main() {
    let _ = FooWithEditContext {
        foo: BazWithEditContext { qux: 0 },
    };
}

uniffi::setup_scaffolding!("wp_derive");
