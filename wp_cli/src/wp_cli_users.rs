use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_i64_or_string;

use crate::run_wp_cli_command;

const USER_FIELDS_ARG: &str = "--fields=ID,user_login,display_name,user_email,user_registered,roles,user_nicename,user_url,user_status,url";

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliUser {
    #[serde(rename = "ID")]
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub id: i64,
    #[serde(rename = "user_login")]
    pub username: String,
    #[serde(rename = "display_name")]
    pub name: String,
    #[serde(rename = "user_email")]
    pub email: String,
    #[serde(rename = "user_registered")]
    pub registered_date: String,
    pub roles: String,
    #[serde(rename = "user_url")]
    pub url: String,
    // Formatted as http://localhost/author/slug/
    #[serde(rename = "url")]
    pub url_slug: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WpCliUserMeta {
    pub user_id: i32,
    pub meta_key: String,
    pub meta_value: String,
}

impl WpCliUser {
    pub fn get(user_id: i64) -> Result<Self> {
        // `wp user get` & `wp user list` returns different fields/information. To avoid this, we
        // always use `wp user list` and then find the user we are interested in.
        Self::list().and_then(|v| {
            v.into_iter()
                .find(|u| u.id == user_id)
                .ok_or(anyhow!("Can't find the user with user_id: {}", user_id,))
        })
    }
    pub fn list() -> Result<Vec<Self>> {
        let output = run_wp_cli_command(["user", "list", USER_FIELDS_ARG]);
        serde_json::from_slice::<Vec<Self>>(&output.stdout)
            .with_context(|| "Failed to parse `wp user list --format=json` into Vec<WpCliUser>")
    }
}

impl WpCliUserMeta {
    pub fn list(user_id: i64) -> Result<Vec<Self>> {
        println!("Fetching user meta: {}", user_id);
        let output = run_wp_cli_command(["user", "meta", "list", &user_id.to_string()]);
        println!("Fetched: {:#?}", String::from_utf8_lossy(&output.stdout));
        serde_json::from_slice::<Vec<Self>>(&output.stdout).with_context(|| {
            format!(
                "Failed to parse `wp user meta list {} --format=json` into WpCliUserMeta",
                user_id
            )
        })
    }
}
