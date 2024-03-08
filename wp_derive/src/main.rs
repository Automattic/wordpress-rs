use wp_derive::EditContext;

#[derive(EditContext)]
pub struct PostObject {
    pub id: Option<u32>,
    pub date: Option<String>,
}

fn main() {}
