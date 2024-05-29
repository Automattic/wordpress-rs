use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct SparseFoo {
    #[WpContext(Edit)]
    pub bar: Option<u32>,
}

fn main() {}

uniffi::setup_scaffolding!();
