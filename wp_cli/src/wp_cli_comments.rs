use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_i64_or_string;

use crate::{run_wp_cli_command, AsWpCliArguments};

const COMMENT_FIELDS_ARG: &str = "--fields=comment_ID,comment_post_ID,comment_author,comment_author_email,comment_author_url,comment_author_IP,comment_date,comment_date_gmt,comment_content,comment_karma,comment_approved,comment_agent,comment_type,comment_parent,user_id";

#[derive(Debug, Default)]
pub struct WpCliCommentListArguments {
    pub comment_status: Option<String>,
}

impl AsWpCliArguments for WpCliCommentListArguments {
    fn as_wp_cli_arguments(&self) -> Option<String> {
        let mut map = HashMap::new();
        if let Some(comment_status) = &self.comment_status {
            map.insert("status", comment_status);
        }
        map.as_wp_cli_arguments()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliComment {
    #[serde(rename = "comment_ID")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub comment_id: i64,
    #[serde(rename = "comment_post_ID")]
    pub comment_post_id: String,
    pub comment_author: String,
    pub comment_author_email: String,
    pub comment_author_url: String,
    #[serde(rename = "comment_author_IP")]
    pub comment_author_ip: String,
    pub comment_date: String,
    pub comment_date_gmt: String,
    pub comment_content: String,
    pub comment_karma: String,
    pub comment_approved: String,
    pub comment_agent: String,
    pub comment_type: String,
    pub comment_parent: String,
    pub user_id: String,
}

impl WpCliComment {
    pub fn get(comment_id: i64) -> Result<Self> {
        let output = run_wp_cli_command([
            "comment",
            "get",
            comment_id.to_string().as_str(),
            COMMENT_FIELDS_ARG,
        ]);
        serde_json::from_slice::<Self>(&output.stdout).with_context(|| {
            "Failed to parse `wp get {comment_id} --format=json` into WpCliComment"
        })
    }
    pub fn list(arguments: Option<WpCliCommentListArguments>) -> Result<Vec<Self>> {
        let output = if let Some(cli_arguments) = arguments.and_then(|a| a.as_wp_cli_arguments()) {
            run_wp_cli_command([
                "comment",
                "list",
                COMMENT_FIELDS_ARG,
                cli_arguments.as_str(),
            ])
        } else {
            run_wp_cli_command(["comment", "list", COMMENT_FIELDS_ARG])
        };
        serde_json::from_slice::<Vec<Self>>(&output.stdout).with_context(|| {
            "Failed to parse `wp comment list --format=json` into
            Vec<WpCliComment>"
        })
    }
}
