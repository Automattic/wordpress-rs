use wp_derive::WpDeriveParamsField;

#[derive(WpDeriveParamsField)]
pub struct PostListParams {
    pub page: Option<u32>,
}

fn main() {}