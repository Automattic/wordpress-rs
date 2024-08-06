use anyhow::{Context, Result};
use std::{collections::HashMap, ffi::OsStr, process::Command};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct WpCliSiteSettingsOption {
    option_name: String,
    option_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliSiteSettings {
    #[serde(rename = "blogname")]
    pub title: Option<String>,
    #[serde(rename = "blogdescription")]
    pub description: Option<String>,
    #[serde(rename = "siteurl")]
    pub url: Option<String>,
    #[serde(rename = "admin_email")]
    pub email: Option<String>,
    #[serde(rename = "timezone_string")]
    pub timezone: Option<String>,
    pub date_format: Option<String>,
    pub time_format: Option<String>,
    pub start_of_week: Option<String>,
    #[serde(rename = "WPLANG")]
    pub language: Option<String>,
    pub use_smilies: Option<String>,
    pub default_category: Option<String>,
    pub default_post_format: Option<String>,
    pub posts_per_page: Option<String>,
    pub show_on_front: Option<String>,
    pub page_on_front: Option<String>,
    pub page_for_posts: Option<String>,
    pub default_ping_status: Option<String>,
    pub default_comment_status: Option<String>,
    pub site_logo: Option<String>,
    pub site_icon: Option<String>,
}

impl WpCliSiteSettings {
    pub fn fetch() -> Result<Self> {
        let output = run_wp_cli_command(["option", "list"]);
        let map = serde_json::from_slice::<Vec<WpCliSiteSettingsOption>>(&output.stdout)
            .with_context(|| {
                "Failed to parse `wp option list --format=json` into Vec<WpCliSiteSettingsOption>"
            })?
            .into_iter()
            .map(|s| (s.option_name, s.option_value))
            .collect::<HashMap<String, String>>();
        serde_json::to_value(map).and_then(Self::deserialize).with_context(|| "Failed to parse `wp option list --format=json` from `HashMap<String, String>` into `WpCliSiteSettings`")
    }
}

fn run_wp_cli_command<I, S>(args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("wp")
        .arg("--allow-root")
        .arg("--http=http://localhost")
        .arg("--path=/var/www/html")
        .arg("--format=json")
        .args(args)
        .output()
        .expect("Failed to run wp-cli command")
}
