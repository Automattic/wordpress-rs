use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_i64_or_string;

use crate::run_wp_cli_command;

const CATEGORIES_FIELDS_ARG: &str = "--fields=term_id,count,description,name,slug,taxonomy,parent";

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliCategory {
    #[serde(rename = "term_id")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub id: i64,
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub count: i64,
    pub description: String,
    pub name: String,
    pub slug: String,
    pub taxonomy: String,
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub parent: i64,
}

impl WpCliCategory {
    pub fn get(category_id: i64) -> Result<Self> {
        let output = run_wp_cli_command([
            "term",
            "get",
            "category",
            category_id.to_string().as_str(),
            CATEGORIES_FIELDS_ARG,
        ]);
        serde_json::from_slice::<Self>(&output.stdout).with_context(|| {
            "Failed to parse `wp term get category {category_id} {CATEGORIES_FIELDS_ARG} --format=json` into `WpCliCategory`"
        })
    }
    pub fn list() -> Result<Vec<Self>> {
        let output = run_wp_cli_command(["term", "list", "category", CATEGORIES_FIELDS_ARG]);
        serde_json::from_slice::<Vec<Self>>(&output.stdout).with_context(
            || "Failed to parse `wp term list category --format=json` into Vec<WpCliCategory>",
        )
    }
}
