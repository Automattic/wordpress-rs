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

    struct Plugin {
        parsed: PluginInformation,
        raw_json: serde_json::Value,
    }

    impl Plugin {
        fn parse(json_string: &str) -> Self {
            let parsed = serde_json::from_str::<PluginInformation>(json_string);
            let raw_json = serde_json::from_str(json_string);
            assert!(parsed.is_ok(), "Failed to parse JSON: {:?}", parsed.err());
            assert!(
                raw_json.is_ok(),
                "Failed to parse JSON: {:?}",
                raw_json.err()
            );

            Self {
                parsed: parsed.unwrap(),
                raw_json: raw_json.unwrap(),
            }
        }
    }

    #[fixture]
    fn plugin_with_variant_types() -> Plugin {
        let json_string = include_str!("../tests/plugin-with-different-types-of-values.json");
        Plugin::parse(json_string)
    }

    #[fixture]
    fn plugin_with_expected_types() -> Plugin {
        let json_string = include_str!("../tests/plugin-with-expected-types.json");
        Plugin::parse(json_string)
    }

    #[rstest]
    fn test_plugin_with_different_types_of_author_profile(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(plugin_with_variant_types.raw_json["author_profile"], false);
        assert_eq!(plugin_with_variant_types.parsed.author_profile, "");

        assert!(plugin_with_expected_types.raw_json["author_profile"].is_string());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_contributors(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(
            plugin_with_variant_types.raw_json["contributors"].as_array(),
            Some(vec![]).as_ref()
        );
        assert_eq!(
            plugin_with_variant_types.parsed.contributors,
            HashMap::new()
        );

        assert!(plugin_with_expected_types.raw_json["contributors"].is_object());
        assert!(!plugin_with_expected_types.parsed.contributors.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_requires(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(plugin_with_variant_types.raw_json["requires"], false);
        assert_eq!(plugin_with_variant_types.parsed.requires, "");

        assert!(plugin_with_expected_types.raw_json["requires"].is_string());
        assert!(!plugin_with_expected_types.parsed.requires.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_tested(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(plugin_with_variant_types.raw_json["tested"], false);
        assert_eq!(plugin_with_variant_types.parsed.tested, "");

        assert!(plugin_with_expected_types.raw_json["tested"].is_string());
        assert!(!plugin_with_expected_types.parsed.tested.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_requires_php(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(plugin_with_variant_types.raw_json["requires_php"], false);
        assert_eq!(plugin_with_variant_types.parsed.requires_php, "");

        assert!(plugin_with_expected_types.raw_json["requires_php"].is_string());
        assert!(!plugin_with_expected_types.parsed.requires_php.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_upgrade_notice(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(
            plugin_with_variant_types.raw_json["upgrade_notice"].as_array(),
            Some(vec![]).as_ref()
        );
        assert_eq!(
            plugin_with_variant_types.parsed.upgrade_notice,
            HashMap::new()
        );

        assert!(plugin_with_expected_types.raw_json["upgrade_notice"].is_object());
        assert!(!plugin_with_expected_types.parsed.upgrade_notice.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_tags(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(
            plugin_with_variant_types.raw_json["tags"].as_array(),
            Some(vec![]).as_ref()
        );
        assert_eq!(plugin_with_variant_types.parsed.tags, HashMap::new());

        assert!(plugin_with_expected_types.raw_json["tags"].is_object());
        assert!(!plugin_with_expected_types.parsed.tags.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_versions(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(
            plugin_with_variant_types.raw_json["versions"].as_array(),
            Some(vec![]).as_ref()
        );
        assert_eq!(plugin_with_variant_types.parsed.versions, HashMap::new());

        assert!(plugin_with_expected_types.raw_json["versions"].is_object());
        assert!(!plugin_with_expected_types.parsed.versions.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_business_model(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(plugin_with_variant_types.raw_json["business_model"], false);
        assert_eq!(plugin_with_variant_types.parsed.business_model, "");

        assert!(plugin_with_expected_types.raw_json["business_model"].is_string());
        assert!(!plugin_with_expected_types.parsed.business_model.is_empty());
    }

    #[rstest]
    fn test_plugin_with_different_types_of_banners(
        plugin_with_variant_types: Plugin,
        plugin_with_expected_types: Plugin,
    ) {
        assert_eq!(
            plugin_with_variant_types.raw_json["banners"].as_array(),
            Some(vec![]).as_ref()
        );
        assert_eq!(plugin_with_variant_types.parsed.banners, Banners::default());

        assert!(plugin_with_expected_types.raw_json["banners"].is_object());
    }
}
