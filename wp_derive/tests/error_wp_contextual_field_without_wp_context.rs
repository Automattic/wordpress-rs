use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext("edit")]
    pub foo: Option<u32>,
    #[WPContextualField]
    pub bar: Option<u32>,
}

fn main() {}

uniffi::setup_scaffolding!("wp_derive");
