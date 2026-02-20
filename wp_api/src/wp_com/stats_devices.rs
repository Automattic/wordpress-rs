use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The time period for grouping device stats.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum StatsDevicesPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsDevicesPeriod);

/// Parameters for the stats devices endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsDevicesParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsDevicesPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The maximum number of entries to return.
    #[uniffi(default = None)]
    pub max: Option<u32>,
    /// The number of periods to include in the response.
    #[uniffi(default = None)]
    pub num: Option<u32>,
    /// The number of days to include in the response.
    #[uniffi(default = None)]
    pub days: Option<u32>,
    /// Whether to return a summary of the data.
    ///
    /// - `true` (default): Response contains aggregated device stats
    ///   across all requested periods.
    /// - `false`: Response contains device stats for the requested periods.
    #[uniffi(default = true)]
    pub summarize: bool,
}

impl Default for StatsDevicesParams {
    fn default() -> Self {
        Self {
            period: None,
            date: None,
            start_date: None,
            max: None,
            num: None,
            days: None,
            summarize: true,
        }
    }
}

impl AppendUrlQueryPairs for StatsDevicesParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_option_query_value_pair("days", self.days.as_ref())
            .append_query_value_pair("summarize", &(self.summarize as u32));
    }
}

/// Response from the stats devices endpoint.
///
/// Contains device type percentages (e.g., desktop, mobile, tablet) keyed by device name.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsDevicesResponse {
    /// Device type percentages keyed by device name (e.g., "desktop", "mobile", "tablet").
    pub top_values: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_devices_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/devices/screensize",
        )
        .expect("Failed to parse url");

        let params = StatsDevicesParams {
            period: Some(StatsDevicesPeriod::Day),
            date: Some("2026-02-20".to_string()),
            start_date: Some("2026-02-14".to_string()),
            max: Some(10),
            num: Some(1),
            days: Some(1),
            summarize: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/devices/screensize?period=day&date=2026-02-20&start_date=2026-02-14&max=10&num=1&days=1&summarize=1"
        );
    }

    #[test]
    fn test_stats_devices_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/devices/screensize",
        )
        .expect("Failed to parse url");

        let params = StatsDevicesParams {
            period: Some(StatsDevicesPeriod::Day),
            date: Some("2026-02-20".to_string()),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        // Default summarize is true, which serializes to summarize=1
        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/devices/screensize?period=day&date=2026-02-20&summarize=1"
        );
    }

    #[test]
    fn test_stats_devices_params_with_false_summarize() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/devices/screensize",
        )
        .expect("Failed to parse url");

        let params = StatsDevicesParams {
            period: Some(StatsDevicesPeriod::Day),
            summarize: false,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/devices/screensize?period=day&summarize=0"
        );
    }

    /// Tests deserialization of all stats devices JSON fixtures.
    #[rstest]
    #[case("tests/wpcom/stats_devices_screensize/summarized-01-day.json")]
    #[case("tests/wpcom/stats_devices_screensize/no-summary-01.json")]
    #[case("tests/wpcom/stats_devices_screensize/summarized-02-day-empty-response.json")]
    #[case("tests/wpcom/stats_devices_screensize/summarized-03-day-all-zero.json")]
    fn test_stats_devices_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let _response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
    }

    #[test]
    fn test_stats_devices_response_deserialization_summary_01() {
        let json_file_path = "tests/wpcom/stats_devices_screensize/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.top_values.len(), 3);
        assert_eq!(response.top_values.get("desktop"), Some(&97.8));
        assert_eq!(response.top_values.get("mobile"), Some(&2.2));
        assert_eq!(response.top_values.get("tablet"), Some(&0.0));
    }

    #[test]
    fn test_stats_devices_empty_response() {
        let json_file_path =
            "tests/wpcom/stats_devices_screensize/summarized-02-day-empty-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.top_values.is_empty());
    }

    #[test]
    fn test_stats_devices_all_zero() {
        let json_file_path = "tests/wpcom/stats_devices_screensize/summarized-03-day-all-zero.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.top_values.len(), 3);
        assert_eq!(response.top_values.get("desktop"), Some(&0.0));
        assert_eq!(response.top_values.get("mobile"), Some(&0.0));
        assert_eq!(response.top_values.get("tablet"), Some(&0.0));
    }

    /// Tests deserialization of all stats devices browser JSON fixtures.
    #[rstest]
    #[case("tests/wpcom/stats_devices_browser/summarized-01-day.json")]
    #[case("tests/wpcom/stats_devices_browser/no-summary-01.json")]
    #[case("tests/wpcom/stats_devices_browser/summarized-02-day-empty-response.json")]
    fn test_stats_devices_browser_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let _response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
    }

    #[test]
    fn test_stats_devices_browser_response_deserialization_summary_01() {
        let json_file_path = "tests/wpcom/stats_devices_browser/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.top_values.len(), 5);
        assert_eq!(response.top_values.get("chrome"), Some(&6431.0));
        assert_eq!(response.top_values.get("firefox"), Some(&563.0));
        assert_eq!(response.top_values.get("safari"), Some(&259.0));
        assert_eq!(response.top_values.get("edge"), Some(&107.0));
        assert_eq!(response.top_values.get("other"), Some(&13.0));
    }

    #[test]
    fn test_stats_devices_browser_empty_response() {
        let json_file_path =
            "tests/wpcom/stats_devices_browser/summarized-02-day-empty-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.top_values.is_empty());
    }

    /// Tests deserialization of all stats devices platform JSON fixtures.
    #[rstest]
    #[case("tests/wpcom/stats_devices_platform/summarized-01-day.json")]
    #[case("tests/wpcom/stats_devices_platform/no-summary-01.json")]
    #[case("tests/wpcom/stats_devices_platform/summarized-02-day-empty-response.json")]
    fn test_stats_devices_platform_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let _response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
    }

    #[test]
    fn test_stats_devices_platform_response_deserialization_summary_01() {
        let json_file_path = "tests/wpcom/stats_devices_platform/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.top_values.len(), 6);
        assert_eq!(response.top_values.get("mac"), Some(&6955.0));
        assert_eq!(response.top_values.get("windows"), Some(&141.0));
        assert_eq!(response.top_values.get("linux"), Some(&113.0));
        assert_eq!(response.top_values.get("android"), Some(&82.0));
        assert_eq!(response.top_values.get("iphone"), Some(&80.0));
        assert_eq!(response.top_values.get("ipad"), Some(&2.0));
    }

    #[test]
    fn test_stats_devices_platform_empty_response() {
        let json_file_path =
            "tests/wpcom/stats_devices_platform/summarized-02-day-empty-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsDevicesResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.top_values.is_empty());
    }
}
