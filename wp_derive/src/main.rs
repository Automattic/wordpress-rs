use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

#[derive(Debug, Serialize, Deserialize, WPContextual, uniffi::Record)]
pub struct PostObject {
    #[serde(rename(serialize = "ser_name"))]
    #[WPContext("edit", "view", "embed")]
    pub id: Option<u32>,
    #[WPContext("edit")]
    pub date: Option<String>,
    #[WPContext("embed")]
    pub embed_date: Option<String>,
    #[WPContext("edit", "view", "embed")]
    pub already_strongly_typed: u32,
}

uniffi::setup_scaffolding!("wp_derive");

fn main() {}
