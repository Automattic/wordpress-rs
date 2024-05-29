// If a field is marked with `#[WpContextualField]` it needs to be a Sparse type

use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(edit)]
    #[WpContextualField]
    bar: Option<Bar>,
}

pub struct Bar {}

fn main() {}

uniffi::setup_scaffolding!();
