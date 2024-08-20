use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_i64_or_string;

use crate::run_wp_cli_command;

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
    pub post_status: String,
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
            run_wp_cli_command(["post", "list", cli_arguments.as_str()])
        } else {
            run_wp_cli_command(["post", "list"])
        };
        serde_json::from_slice::<Vec<Self>>(&output.stdout)
            .with_context(|| "Failed to parse `wp post list --format=json` into Vec<WpCliPost>")
    }
}
