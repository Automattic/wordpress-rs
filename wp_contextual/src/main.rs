use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostObject {
    #[serde(rename(serialize = "ser_name"))]
    #[WpContext(edit, view, embed)]
    pub id: Option<u32>,
    #[WpContext(edit)]
    pub date: Option<String>,
    #[WpContext(embed)]
    pub embed_date: Option<String>,
    #[WpContext(edit, view, embed)]
    pub already_strongly_typed: u32,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<SparsePostGuid>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PostGuid {
    pub raw: Option<String>,
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostGuid {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
}

uniffi::setup_scaffolding!();

fn main() {}
