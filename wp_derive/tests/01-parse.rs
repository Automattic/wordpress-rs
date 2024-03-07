use wp_derive::Contextual;

#[derive(Contextual)]
pub struct PostObject {
    pub id: Option<u32>,
    pub date: Option<String>,
}

fn main() {}
