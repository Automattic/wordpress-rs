use wp_contextual::WpContextual;

#[derive(WpContextual)]
pub struct Foo {}

fn main() {}

uniffi::setup_scaffolding!();
