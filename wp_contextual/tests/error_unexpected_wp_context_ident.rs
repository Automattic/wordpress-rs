use wp_contextual::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(Edit)]
    pub bar: Option<u32>,
}

fn main() {}

uniffi::setup_scaffolding!();
