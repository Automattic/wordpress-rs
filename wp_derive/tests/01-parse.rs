use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct PostObject {
    pub id: Option<u32>,
    pub date: Option<String>,
}

fn main() {}
