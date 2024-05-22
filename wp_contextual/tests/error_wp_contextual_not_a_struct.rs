use wp_contextual::WPContextual;

#[derive(WPContextual)]
pub enum SparseFoo {}

fn main() {}

uniffi::setup_scaffolding!();
