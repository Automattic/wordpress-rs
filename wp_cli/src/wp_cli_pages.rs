use crate::{AsWpCliArguments, run_wp_cli_command};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::deserialize_i64_or_string;

const PAGE_FIELDS_ARG: &str = "--fields=ID,post_name,post_title,post_date,post_status,post_author,post_date_gmt,post_content,post_excerpt,comment_status,ping_status,post_password,post_modified,post_modified_gmt,guid,post_type,post_parent,menu_order";

#[derive(Debug, Default)]
pub struct WpCliPageListArguments {
    pub post_status: Option<String>,
}

impl AsWpCliArguments for WpCliPageListArguments {
    fn as_wp_cli_arguments(&self) -> Option<String> {
        let mut map = HashMap::new();
        if let Some(post_status) = &self.post_status {
            map.insert("post_status", post_status);
        }
        map.as_wp_cli_arguments()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliPage {
    #[serde(rename = "ID")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub id: i64,
    #[serde(rename = "post_author")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub author: i64,
    pub comment_status: String,
    #[serde(rename = "post_content")]
    pub content: String,
    #[serde(rename = "post_date")]
    pub date: String,
    #[serde(rename = "post_date_gmt")]
    pub date_gmt: String,
    #[serde(rename = "post_excerpt")]
    pub excerpt: String,
    pub guid: String,
    #[serde(rename = "menu_order")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub menu_order: i64,
    #[serde(rename = "post_modified")]
    pub modified: String,
    #[serde(rename = "post_modified_gmt")]
    pub modified_gmt: String,
    #[serde(rename = "post_parent")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub parent: i64,
    #[serde(rename = "post_password")]
    pub password: String,
    pub ping_status: String,
    pub post_status: String,
    pub post_type: String,
    #[serde(rename = "post_name")]
    pub slug: String,
    #[serde(rename = "post_title")]
    pub title: String,
}

impl WpCliPage {
    pub fn get(page_id: i64) -> Result<Self> {
        // Some `wp` commands return different fields/information for `get` or `list`. To avoid
        // this, always use `wp post list --post_type=page` and then find the page we are interested in.
        Self::list(None).and_then(|v| {
            v.into_iter()
                .find(|u| u.id == page_id)
                .ok_or(anyhow!("Can't find the page with page_id: {}", page_id,))
        })
    }
    pub fn list(arguments: Option<WpCliPageListArguments>) -> Result<Vec<Self>> {
        let output = if let Some(cli_arguments) = arguments.and_then(|a| a.as_wp_cli_arguments()) {
            run_wp_cli_command([
                "post",
                "list",
                "--post_type=page",
                PAGE_FIELDS_ARG,
                cli_arguments.as_str(),
            ])
        } else {
            run_wp_cli_command(["post", "list", "--post_type=page", PAGE_FIELDS_ARG])
        };
        serde_json::from_slice::<Vec<Self>>(&output.stdout).with_context(
            || "Failed to parse `wp post list --post_type=page --format=json` into Vec<WpCliPage>",
        )
    }
}
