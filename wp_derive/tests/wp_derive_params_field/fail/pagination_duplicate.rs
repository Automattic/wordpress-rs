use wp_derive::WpDeriveParamsField;

#[derive(WpDeriveParamsField)]
#[supports_pagination(true)]
#[supports_pagination(false)] // Duplicate should fail
pub struct TestParams {
    pub page: Option<u32>,
}

fn main() {}