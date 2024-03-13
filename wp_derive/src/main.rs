use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePostObject {
    #[serde(rename(serialize = "ser_name"))]
    #[WPContext("edit", "view", "embed")]
    pub id: Option<u32>,
    #[WPContext("edit")]
    pub date: Option<String>,
    #[WPContext("embed")]
    pub embed_date: Option<String>,
    #[WPContext("edit", "view", "embed")]
    pub already_strongly_typed: u32,
    #[WPContext("edit", "view")]
    #[WPContextualField]
    pub guid: Option<SparsePostGuid>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparsePostGuid {
    #[WPContext("edit")]
    pub raw: Option<String>,
    #[WPContext("edit", "view")]
    pub rendered: Option<String>,
}

uniffi::setup_scaffolding!("wp_derive");

fn main() {}
