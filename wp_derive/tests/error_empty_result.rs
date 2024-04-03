use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct SparseFoo {}

fn main() {}

uniffi::setup_scaffolding!();
