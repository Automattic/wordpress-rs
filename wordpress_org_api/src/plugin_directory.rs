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
    pub support_url: String,
    pub support_threads: u32,
    pub support_threads_resolved: u32,
    pub active_installs: u64,
    pub last_updated: String,
    pub added: String,
    pub homepage: String,
    pub sections: HashMap<String, String>,
    pub download_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub upgrade_notice: HashMap<String, String>,
    pub screenshots: Screenshots,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub tags: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub versions: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub business_model: String,
    pub repository_url: String,
    pub commercial_support_url: String,
    pub donate_link: String,
    #[serde(deserialize_with = "deserialize_default_values")]
    pub banners: Banners,
    pub icons: Option<Icons>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    const JSON_STRING: &str = include_str!("../tests/plugin-with-different-types-of-values.json");

    #[fixture]
    fn plugin_info() -> PluginInformation {
        let result = serde_json::from_str::<PluginInformation>(JSON_STRING);
        assert!(result.is_ok());
        result.unwrap()
    }

    #[fixture]
    fn raw_json() -> serde_json::Value {
        let result = serde_json::from_str(JSON_STRING);
        assert!(result.is_ok());
        result.unwrap()
    }

    #[rstest]
    fn test_plugin_with_different_types_of_author_profile(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["author_profile"], false);
        assert_eq!(plugin_info.author_profile, "");
    }

    #[rstest]
    fn test_plugin_with_different_types_of_contributors(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["contributors"].as_array(), Some(vec![]).as_ref());
        assert_eq!(plugin_info.contributors, HashMap::new());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_requires(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["requires"], false);
        assert_eq!(plugin_info.requires, "");
    }

    #[rstest]
    fn test_plugin_with_different_types_of_tested(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["tested"], false);
        assert_eq!(plugin_info.tested, "");
    }

    #[rstest]
    fn test_plugin_with_different_types_of_requires_php(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["requires_php"], false);
        assert_eq!(plugin_info.requires_php, "");
    }

    #[rstest]
    fn test_plugin_with_different_types_of_upgrade_notice(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["upgrade_notice"].as_array(), Some(vec![]).as_ref());
        assert_eq!(plugin_info.upgrade_notice, HashMap::new());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_tags(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["tags"].as_array(), Some(vec![]).as_ref());
        assert_eq!(plugin_info.tags, HashMap::new());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_versions(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["versions"].as_array(), Some(vec![]).as_ref());
        assert_eq!(plugin_info.versions, HashMap::new());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_business_model(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["business_model"], false);
        assert_eq!(plugin_info.business_model, "");
    }

    #[rstest]
    fn test_plugin_with_different_types_of_banners(
        plugin_info: PluginInformation,
        raw_json: serde_json::Value,
    ) {
        assert_eq!(raw_json["banners"].as_array(), Some(vec![]).as_ref());
        assert_eq!(plugin_info.banners, Banners::default());
    }
}
