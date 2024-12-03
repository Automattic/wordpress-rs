#![allow(dead_code)]

use serde::Deserialize;

use std::{collections::HashMap, fmt::Debug};

use crate::de::deserialize_default_values;

#[derive(Deserialize, Debug)]
pub struct PluginInformation {
    pub name: String,
    pub slug: String,
    pub version: String,
    pub author: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub author_profile: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    #[serde(default)]
    pub contributors: HashMap<String, ContributorDetails>,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub requires: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub tested: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub requires_php: String,
    pub requires_plugins: Vec<String>,
    pub rating: u32,
    pub ratings: Ratings,
    pub num_ratings: u32,
    #[serde(default)]
    pub support_url: String,
    pub support_threads: u32,
    pub support_threads_resolved: u32,
    pub active_installs: u64,
    pub last_updated: String,
    pub added: String,
    pub homepage: String,
    #[serde(default)]
    pub sections: HashMap<String, String>,
    pub download_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    #[serde(default)]
    pub upgrade_notice: HashMap<String, String>,
    #[serde(default)]
    pub screenshots: Screenshots,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub tags: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    #[serde(default)]
    pub versions: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    #[serde(default)]
    pub business_model: String,
    #[serde(default)]
    pub repository_url: String,
    #[serde(default)]
    pub commercial_support_url: String,
    pub donate_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    #[serde(default)]
    pub banners: Banners,
    pub icons: Option<Icons>,
    #[serde(default)]
    pub preview_link: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct ContributorDetails {
    pub profile: String,
    pub avatar: String,
    pub display_name: String,
}

#[derive(Deserialize, Debug)]
pub struct Ratings {
    #[serde(rename = "5")]
    pub five_star: u32,
    #[serde(rename = "4")]
    pub four_star: u32,
    #[serde(rename = "3")]
    pub three_star: u32,
    #[serde(rename = "2")]
    pub two_star: u32,
    #[serde(rename = "1")]
    pub one_star: u32,
}

/// https://developer.wordpress.org/plugins/wordpress-org/plugin-assets/#screenshots
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Screenshots {
    Named(HashMap<String, Screenshot>),
    List(Vec<Screenshot>),
}

impl Default for Screenshots {
    fn default() -> Self {
        Screenshots::List(vec![])
    }
}

#[derive(Deserialize, Debug)]
pub struct Screenshot {
    pub src: String,
    pub caption: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Default)]
pub struct Banners {
    #[serde(deserialize_with = "deserialize_default_values")]
    pub low: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub high: String,
}

#[derive(Deserialize, Debug)]
pub struct Icons {
    #[serde(rename = "1x")]
    pub low: Option<String>,
    #[serde(rename = "2x")]
    pub high: Option<String>,
    pub svg: Option<String>,
    pub default: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct QueryPluginResponse {
    pub info: QueryPluginResponseInfo,
    pub plugins: Vec<PluginInformation>,
}

#[derive(Deserialize, Debug)]
pub struct QueryPluginResponseInfo {
    pub page: i64,
    pub pages: i64,
    pub results: i64,
}
