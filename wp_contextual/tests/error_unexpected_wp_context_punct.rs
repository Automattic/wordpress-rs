use wp_contextual::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {
    #[WPContext(edit. view)]
    pub bar: Option<u32>,
}

fn main() {}

uniffi::setup_scaffolding!();
