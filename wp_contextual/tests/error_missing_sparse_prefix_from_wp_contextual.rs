use wp_contextual::WPContextual;

#[derive(WPContextual)]
pub struct Foo {}

fn main() {}

uniffi::setup_scaffolding!();
