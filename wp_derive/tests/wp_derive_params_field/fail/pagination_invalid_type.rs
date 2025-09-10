use wp_derive::WpDeriveParamsField;

#[derive(WpDeriveParamsField)]
#[supports_pagination("not_a_bool")] // Should be true or false, not a string
pub struct TestParams {
    pub page: Option<u32>,
}

fn main() {}