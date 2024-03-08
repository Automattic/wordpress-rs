use wp_derive::WPContextual;

#[derive(WPContextual)]
pub struct PostObject {
    #[WPContext("edit", "view", "embed")]
    pub id: Option<u32>,
    #[WPContext("edit")]
    pub date: Option<String>,
    #[WPContext("embed")]
    pub embed_date: Option<String>,
    #[WPContext("edit", "view", "embed")]
    pub already_strongly_typed: u32,
}

fn main() {}
