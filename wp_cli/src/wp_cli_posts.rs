use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_i64_or_string;

use crate::run_wp_cli_command;

const POST_FIELDS_ARG: &str = "--fields=ID, post_title, post_date, post_status, post_author, post_date_gmt, post_content, post_excerpt, comment_status, ping_status, post_password, post_modified, post_modified_gmt, guid, post_type";

#[derive(Debug, Default)]
pub struct WpCliPostListArguments {
    pub post_status: Option<String>,
}

impl WpCliPostListArguments {
    fn as_wp_cli_arguments(&self) -> Option<String> {
        let mut s = String::new();
        Self::add_field_arg(&mut s, "post_status", self.post_status.as_ref());
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    fn add_field_arg(args: &mut String, field_name: &str, field: Option<&String>) {
        if let Some(f) = field {
            args.push_str(format!("--{}={}", field_name, f).as_str());
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliPost {
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
    #[serde(rename = "post_modified")]
    pub modified: String,
    #[serde(rename = "post_modified_gmt")]
    pub modified_gmt: String,
    #[serde(rename = "post_password")]
    pub password: String,
    pub ping_status: String,
    pub post_status: String,
    pub post_type: String,
    #[serde(rename = "post_title")]
    pub title: String,
}

impl WpCliPost {
    pub fn get(post_id: i64) -> Result<Self> {
        // Some `wp` commands return different fields/information for `get` or `list`. To avoid
        // this, always use `wp post list` and then find the post we are interested in.
        Self::list(None).and_then(|v| {
            v.into_iter()
                .find(|u| u.id == post_id)
                .ok_or(anyhow!("Can't find the post with post_id: {}", post_id,))
        })
    }
    pub fn list(arguments: Option<WpCliPostListArguments>) -> Result<Vec<Self>> {
        let output = if let Some(cli_arguments) = arguments.and_then(|a| a.as_wp_cli_arguments()) {
            run_wp_cli_command(["post", "list", POST_FIELDS_ARG, cli_arguments.as_str()])
        } else {
            run_wp_cli_command(["post", "list", POST_FIELDS_ARG])
        };
        serde_json::from_slice::<Vec<Self>>(&output.stdout)
            .with_context(|| "Failed to parse `wp post list --format=json` into Vec<WpCliPost>")
    }
}
