use wp_derive::WPContextual;

#[derive(WPContextual)]
pub enum SparseFoo {}

fn main() {}

uniffi::setup_scaffolding!();
