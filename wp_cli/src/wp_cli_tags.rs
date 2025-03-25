use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_i64_or_string;

use crate::run_wp_cli_command;

const TAGS_FIELDS_ARG: &str = "--fields=term_id,count,description,name,slug,taxonomy";

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliTag {
    #[serde(rename = "term_id")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub id: i64,
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub count: i64,
    pub description: String,
    pub name: String,
    pub slug: String,
    pub taxonomy: String,
}

impl WpCliTag {
    pub fn get(tag_id: i64) -> Result<Self> {
        let output = run_wp_cli_command([
            "term",
            "get",
            "post_tag",
            tag_id.to_string().as_str(),
            TAGS_FIELDS_ARG,
        ]);
        serde_json::from_slice::<Self>(&output.stdout).with_context(|| {
            "Failed to parse `wp term get post_tag {tag_id} {TAGS_FIELDS_ARG} --format=json` into `WpCliTag`"
        })
    }
    pub fn list() -> Result<Vec<Self>> {
        let output = run_wp_cli_command(["term", "list", "post_tag", TAGS_FIELDS_ARG]);
        serde_json::from_slice::<Vec<Self>>(&output.stdout).with_context(
            || "Failed to parse `wp term list post_tag --format=json` into Vec<WpCliTag>",
        )
    }
}
