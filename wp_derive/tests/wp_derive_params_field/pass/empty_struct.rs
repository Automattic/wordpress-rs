use wp_derive::WpDeriveParamsField;

#[derive(WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct EmptyParams {}

fn main() {}