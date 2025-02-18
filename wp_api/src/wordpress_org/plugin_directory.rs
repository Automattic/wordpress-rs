use super::de::deserialize_default_values;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};

#[derive(Serialize, Deserialize, Debug, uniffi::Record)]
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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, uniffi::Record)]
pub struct ContributorDetails {
    pub profile: String,
    pub avatar: String,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug, uniffi::Record)]
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
#[derive(Serialize, Deserialize, Debug, uniffi::Enum)]
#[serde(untagged)]
pub enum Screenshots {
    Named(HashMap<String, Screenshot>),
    Unnamed(Vec<Screenshot>),
}

impl Default for Screenshots {
    fn default() -> Self {
        Screenshots::Unnamed(vec![])
    }
}

#[derive(Serialize, Deserialize, Debug, uniffi::Record)]
pub struct Screenshot {
    pub src: String,
    pub caption: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default, uniffi::Record)]
pub struct Banners {
    #[serde(deserialize_with = "deserialize_default_values", alias = "1x")]
    pub low: String,
    #[serde(deserialize_with = "deserialize_default_values", alias = "2x")]
    pub high: String,
}

#[derive(Serialize, Deserialize, Debug, uniffi::Record)]
pub struct Icons {
    #[serde(rename = "1x")]
    pub low: Option<String>,
    #[serde(rename = "2x")]
    pub high: Option<String>,
    pub svg: Option<String>,
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, uniffi::Record)]
pub struct QueryPluginResponse {
    pub info: QueryPluginResponseInfo,
    pub plugins: Vec<PluginInformation>,
}

#[derive(Serialize, Deserialize, Debug, uniffi::Record)]
pub struct QueryPluginResponseInfo {
    pub page: u64,
    pub pages: u64,
    pub results: u64,
}

crate::uniffi_export_serialization!(plugin_information, PluginInformation);
crate::uniffi_export_serialization!(plugin_information_list, Vec<PluginInformation>);

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
        let json_string =
            include_str!("../../tests/plugin-directory/plugin-with-different-types-of-values.json");
        Plugin::parse(json_string)
    }

    #[fixture]
    fn plugin_with_expected_types() -> Plugin {
        let json_string =
            include_str!("../../tests/plugin-directory/plugin-with-expected-types.json");
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
        assert!(!plugin_with_expected_types.parsed.author_profile.is_empty());
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

    #[test]
    fn test_plugin_query_result() {
        let json_string = include_str!("../../tests/plugin-directory/plugin-query-result.json");
        let parsed = serde_json::from_str::<QueryPluginResponse>(json_string);
        assert!(parsed.is_ok(), "Failed to parse JSON: {:?}", parsed.err());
    }

    #[test]
    fn plugin_directory_single_plugin_case_1() {
        let json_string =
            include_str!("../../tests/plugin-directory/plugin_directory_single_plugin_case_1.json");
        let parsed = serde_json::from_str::<PluginInformation>(json_string);
        assert!(parsed.is_ok(), "Failed to parse JSON: {:?}", parsed.err());
    }

    #[test]
    fn serialization_round_trip() {
        let json_string =
            include_str!("../../tests/plugin-directory/plugin-with-expected-types.json");
        let plugin = serde_json::from_str::<PluginInformation>(json_string).unwrap();
        let serialized = serialize_plugin_information(plugin).unwrap();
        let deserialized = deserialize_plugin_information(serialized).unwrap();

        let expected = serde_json::from_str::<PluginInformation>(json_string).unwrap();
        assert_eq!(deserialized.name, expected.name);
        assert_eq!(deserialized.author, expected.author);
    }

    #[test]
    fn serialization_list_round_trip() {
        let json_string =
            include_str!("../../tests/plugin-directory/plugin-with-expected-types.json");
        let plugin = serde_json::from_str::<PluginInformation>(json_string).unwrap();
        let serialized = serialize_plugin_information_list(vec![plugin]).unwrap();
        let deserialized = deserialize_plugin_information_list(serialized)
            .unwrap()
            .pop()
            .unwrap();

        let expected = serde_json::from_str::<PluginInformation>(json_string).unwrap();
        assert_eq!(deserialized.name, expected.name);
        assert_eq!(deserialized.author, expected.author);
    }
}
