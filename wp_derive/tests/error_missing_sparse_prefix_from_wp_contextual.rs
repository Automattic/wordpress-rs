use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct Foo {}

fn main() {}

uniffi::setup_scaffolding!("wp_derive");
