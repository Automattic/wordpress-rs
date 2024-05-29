use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub enum SparseFoo {}

fn main() {}

uniffi::setup_scaffolding!();
