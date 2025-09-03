use wp_derive::WpDeriveParamsField;

#[derive(WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct TestParams {
    #[field_name("per_page")]
    #[field_name("per_page")] // Duplicate should fail
    pub per_page: Option<u32>,
}

fn main() {}
