// If a field is marked with `#[WPContextualField]` it needs to be a Sparse type

use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext("edit")]
    #[WPContextualField]
    bar: Option<Bar>,
}

pub struct Bar {}

fn main() {}

uniffi::setup_scaffolding!("wp_derive");
