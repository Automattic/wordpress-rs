use crate::url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::deserialize_empty_array_or_hashmap;

/// A single UTM key that can be used in the stats UTM endpoint path.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
pub enum StatsUtmKey {
    #[strum(serialize = "utm_source")]
    #[serde(rename = "utm_source")]
    UtmSource,
    #[strum(serialize = "utm_medium")]
    #[serde(rename = "utm_medium")]
    UtmMedium,
    #[strum(serialize = "utm_campaign")]
    #[serde(rename = "utm_campaign")]
    UtmCampaign,
}

uniffi::custom_newtype!(StatsUtmKeys, String);
/// Comma-separated UTM keys used as a URL path segment.
///
/// For example, `"utm_source,utm_medium"` or `"utm_campaign"`.
/// Construct from a list of `StatsUtmKey` using `StatsUtmKeys::new`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsUtmKeys(pub String);

impl StatsUtmKeys {
    pub fn new(keys: &[StatsUtmKey]) -> Self {
        assert!(!keys.is_empty(), "At least one UTM key must be provided");
        Self(
            keys.iter()
                .map(|k| k.to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

impl std::fmt::Display for StatsUtmKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parameters for the stats UTM endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsUtmParams {
    /// The maximum number of results to return. Use 0 for unlimited.
    #[uniffi(default = None)]
    pub max: Option<u32>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The number of days to include in the query.
    #[uniffi(default = None)]
    pub days: Option<u32>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// Whether to include top posts data in the response.
    #[uniffi(default = true)]
    pub query_top_posts: bool,
}

impl Default for StatsUtmParams {
    fn default() -> Self {
        Self {
            max: None,
            date: None,
            days: None,
            start_date: None,
            query_top_posts: true,
        }
    }
}

impl AppendUrlQueryPairs for StatsUtmParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("days", self.days.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_query_value_pair("query_top_posts", &(self.query_top_posts as u32));
    }
}

/// Response from the stats UTM endpoint.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsUtmResponse {
    /// Top UTM values with their view counts.
    /// Keys are UTM value strings (single values like `"impact"` or JSON arrays
    /// like `["impact","affiliate"]` when multiple UTM keys are queried).
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub top_utm_values: HashMap<String, u64>,
    /// Top posts grouped by UTM value.
    /// Keys match the keys in `top_utm_values`.
    #[serde(default, deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub top_posts: HashMap<String, Vec<StatsUtmPost>>,
}

/// A post entry in the stats UTM response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsUtmPost {
    /// The post ID. 0 indicates the home page or archives.
    pub id: u64,
    /// The URL of the post.
    pub href: String,
    /// The title of the post.
    pub title: String,
    /// The number of views from this UTM source.
    pub views: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_utm_keys_single() {
        let keys = StatsUtmKeys::new(&[StatsUtmKey::UtmSource]);
        assert_eq!(keys.to_string(), "utm_source");
    }

    #[test]
    fn test_stats_utm_keys_multiple() {
        let keys = StatsUtmKeys::new(&[StatsUtmKey::UtmSource, StatsUtmKey::UtmMedium]);
        assert_eq!(keys.to_string(), "utm_source,utm_medium");
    }

    #[test]
    fn test_stats_utm_keys_all() {
        let keys = StatsUtmKeys::new(&[
            StatsUtmKey::UtmCampaign,
            StatsUtmKey::UtmSource,
            StatsUtmKey::UtmMedium,
        ]);
        assert_eq!(keys.to_string(), "utm_campaign,utm_source,utm_medium");
    }

    #[test]
    #[should_panic(expected = "At least one UTM key must be provided")]
    fn test_stats_utm_keys_empty() {
        StatsUtmKeys::new(&[]);
    }

    #[test]
    fn test_stats_utm_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/utm/utm_source",
        )
        .expect("Failed to parse url");

        let params = StatsUtmParams {
            max: Some(0),
            date: Some("2026-03-24".to_string()),
            days: Some(365),
            start_date: Some("2026-03-24".to_string()),
            query_top_posts: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/utm/utm_source?max=0&date=2026-03-24&days=365&start_date=2026-03-24&query_top_posts=1"
        );
    }

    #[test]
    fn test_stats_utm_params_serialization_minimal() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/utm/utm_source",
        )
        .expect("Failed to parse url");

        let params = StatsUtmParams {
            date: Some("2026-03-24".to_string()),
            days: Some(1),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/utm/utm_source?date=2026-03-24&days=1&query_top_posts=1"
        );
    }

    #[test]
    fn test_stats_utm_params_with_false_query_top_posts() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/utm/utm_source",
        )
        .expect("Failed to parse url");

        let params = StatsUtmParams {
            query_top_posts: false,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/utm/utm_source?query_top_posts=0"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_utm/single-key.json")]
    #[case("tests/wpcom/stats_utm/multiple-keys.json")]
    #[case("tests/wpcom/stats_utm/triple-keys.json")]
    fn test_stats_utm_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsUtmResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.top_utm_values.is_empty());
        assert!(!response.top_posts.is_empty());

        // Keys in top_utm_values and top_posts should match
        for key in response.top_utm_values.keys() {
            assert!(
                response.top_posts.contains_key(key),
                "top_posts should contain key: {key}"
            );
        }
    }

    #[test]
    fn test_stats_utm_response_deserialization_single_key() {
        let json_file_path = "tests/wpcom/stats_utm/single-key.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsUtmResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.top_utm_values.len(), 3);
        assert_eq!(response.top_utm_values["impact"], 5);
        assert_eq!(response.top_utm_values["trustpilot"], 1);
        assert_eq!(response.top_utm_values["hovercard"], 1);

        let impact_posts = &response.top_posts["impact"];
        assert_eq!(impact_posts.len(), 2);
        assert_eq!(impact_posts[0].id, 146836);
        assert_eq!(impact_posts[0].views, 3);
    }

    #[test]
    fn test_stats_utm_response_deserialization_empty() {
        let json_file_path = "tests/wpcom/stats_utm/empty-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsUtmResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.top_utm_values.is_empty());
        assert!(response.top_posts.is_empty());
    }

    #[test]
    fn test_stats_utm_response_deserialization_empty_arrays() {
        let json_file_path = "tests/wpcom/stats_utm/empty-arrays-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsUtmResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.top_utm_values.is_empty());
        assert!(response.top_posts.is_empty());
    }

    #[test]
    fn test_stats_utm_response_deserialization_missing_top_posts() {
        let json_file_path = "tests/wpcom/stats_utm/missing-top-posts.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsUtmResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.top_utm_values.is_empty());
        assert!(response.top_posts.is_empty());
    }
}
