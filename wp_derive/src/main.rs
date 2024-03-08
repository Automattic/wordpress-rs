use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct PostObject {
    #[WPContext("edit", "view", "embed")]
    pub id: Option<u32>,
    #[WPContext("edit")]
    pub date: Option<String>,
}

fn main() {}
