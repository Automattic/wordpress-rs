use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::deserialize_option_empty_array_or_hashmap;

/// The time period for grouping country views.
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
pub enum StatsCountryViewsPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsCountryViewsPeriod);

/// Parameters for the stats country views endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsCountryViewsParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsCountryViewsPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The maximum number of countries to return.
    #[uniffi(default = None)]
    pub max: Option<u32>,
    /// The number of periods to include in the response.
    #[uniffi(default = None)]
    pub num: Option<u32>,
    /// The number of days to include in the response.
    #[uniffi(default = None)]
    pub days: Option<u32>,
    /// The locale for the response.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
    /// Whether to return a summary of the data.
    ///
    /// - `Some(true)` (summarize=1): Response contains `summary` field with aggregated country
    ///   views across all requested periods. The `days` field will be absent.
    /// - `Some(false)` (summarize=0, default): Response contains `days` field with a per-day
    ///   breakdown of country views, where each day is keyed by its date string. The `summary`
    ///   field will be absent.
    #[uniffi(default = Some(false))]
    pub summarize: Option<bool>,
}

impl Default for StatsCountryViewsParams {
    fn default() -> Self {
        Self {
            period: None,
            date: None,
            start_date: None,
            max: None,
            num: None,
            days: None,
            locale: None,
            summarize: Some(false),
        }
    }
}

impl AppendUrlQueryPairs for StatsCountryViewsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_option_query_value_pair("days", self.days.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_option_query_value_pair("summarize", self.summarize.map(|b| b as u32).as_ref());
    }
}

/// Response from the stats country views endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize` is not set: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsCountryViewsResponse {
    /// The date for the stats query.
    pub date: String,
    /// Country information keyed by country code.
    /// Can be `null`, an empty array `[]`, or a map of country codes to info.
    #[serde(
        rename = "country-info",
        deserialize_with = "deserialize_option_empty_array_or_hashmap"
    )]
    pub country_info: Option<HashMap<String, StatsCountryInfo>>,
    /// Summary data with aggregated country views (present when summarize=1).
    pub summary: Option<StatsCountryViewsSummaryData>,
    /// Per-day stats data keyed by date string (present when summarize is not set).
    pub days: Option<HashMap<String, StatsCountryViewsDayData>>,
}

/// Country information including flag icons and display name.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsCountryInfo {
    /// The URL of the flag icon.
    pub flag_icon: Option<String>,
    /// The URL of the flat flag icon.
    pub flat_flag_icon: Option<String>,
    /// The full country name.
    pub country_full: Option<String>,
    /// The map region code.
    pub map_region: Option<String>,
}

/// Summary data with aggregated country views.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsCountryViewsSummaryData {
    /// The list of country views.
    pub views: Vec<StatsCountryView>,
    /// Views from other countries not included in the list.
    pub other_views: u64,
    /// The total number of views.
    pub total_views: u64,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsCountryViewsDayData {
    /// The list of country views for this day.
    pub views: Vec<StatsCountryView>,
    /// Views from other countries not included in the list.
    pub other_views: u64,
    /// The total number of views for this day.
    pub total_views: u64,
}

/// A country view entry in the stats country views response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsCountryView {
    /// The location/country name.
    pub location: Option<String>,
    /// The number of views from this country.
    pub views: Option<u64>,
    /// The country code.
    pub country_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_country_views_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views",
        )
        .expect("Failed to parse url");

        let params = StatsCountryViewsParams {
            period: Some(StatsCountryViewsPeriod::Day),
            date: Some("2026-01-29".to_string()),
            start_date: Some("2026-01-29".to_string()),
            max: Some(10),
            num: Some(1),
            days: Some(1),
            locale: Some(WPComLanguage::English),
            summarize: Some(true),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views?period=day&date=2026-01-29&start_date=2026-01-29&max=10&num=1&days=1&locale=en&summarize=1"
        );
    }

    #[test]
    fn test_stats_country_views_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views",
        )
        .expect("Failed to parse url");

        let params = StatsCountryViewsParams {
            period: Some(StatsCountryViewsPeriod::Day),
            date: Some("2026-01-29".to_string()),
            start_date: Some("2026-01-23".to_string()),
            locale: Some(WPComLanguage::English),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        // Default summarize is Some(false), which serializes to summarize=0
        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views?period=day&date=2026-01-29&start_date=2026-01-23&locale=en&summarize=0"
        );
    }

    #[test]
    fn test_stats_country_views_params_without_summarize() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views",
        )
        .expect("Failed to parse url");

        let params = StatsCountryViewsParams {
            period: Some(StatsCountryViewsPeriod::Day),
            date: Some("2026-01-29".to_string()),
            summarize: None,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views?period=day&date=2026-01-29"
        );
    }

    #[test]
    fn test_stats_country_views_params_with_false_summarize() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views",
        )
        .expect("Failed to parse url");

        let params = StatsCountryViewsParams {
            period: Some(StatsCountryViewsPeriod::Day),
            summarize: Some(false),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/country-views?period=day&summarize=0"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_country_views/country-views-01-summary.json")]
    #[case("tests/wpcom/stats_country_views/country-views-02-days.json")]
    #[case("tests/wpcom/stats_country_views/country-views-03-with-nulls.json")]
    #[case("tests/wpcom/stats_country_views/country-views-04-empty.json")]
    #[case("tests/wpcom/stats_country_views/country-views-05-empty-null.json")]
    fn test_stats_country_views_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsCountryViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
    }

    #[test]
    fn test_stats_country_views_response_deserialization_summary_01() {
        let json_file_path = "tests/wpcom/stats_country_views/country-views-01-summary.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsCountryViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-29");
        assert!(response.summary.is_some());
        assert!(response.days.is_none());

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_views, 0);
        assert_eq!(summary.other_views, 0);
        assert_eq!(summary.views.len(), 22);

        // Verify first country view
        let first_view = &summary.views[0];
        assert_eq!(first_view.location, Some("United States".to_string()));
        assert_eq!(first_view.views, Some(228));
        assert_eq!(first_view.country_code, Some("US".to_string()));

        // Verify country info
        let country_info = response
            .country_info
            .as_ref()
            .expect("Country info should be present");
        assert!(country_info.contains_key("US"));

        let us_info = country_info.get("US").expect("US info should exist");
        assert_eq!(us_info.country_full, Some("United States".to_string()));
        assert!(us_info.flag_icon.is_some());
        assert!(us_info.flat_flag_icon.is_some());
        assert_eq!(us_info.map_region, Some("021".to_string()));
    }

    #[test]
    fn test_stats_country_views_response_deserialization_days_02() {
        let json_file_path = "tests/wpcom/stats_country_views/country-views-02-days.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsCountryViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-29");
        assert!(response.summary.is_none());
        assert!(response.days.is_some());

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 7);

        // Verify first day
        let day1 = days.get("2026-01-29").expect("2026-01-29 should exist");
        assert_eq!(day1.total_views, 525);
        assert_eq!(day1.other_views, 61);
        assert_eq!(day1.views.len(), 10);

        // Verify first country view of first day
        let first_view = &day1.views[0];
        assert_eq!(first_view.location, Some("United States".to_string()));
        assert_eq!(first_view.views, Some(228));
        assert_eq!(first_view.country_code, Some("US".to_string()));

        // Verify country info
        let country_info = response
            .country_info
            .as_ref()
            .expect("Country info should be present");
        assert!(country_info.contains_key("US"));
        assert!(country_info.contains_key("AU"));
    }

    #[test]
    fn test_stats_country_views_with_null_values() {
        let json_file_path = "tests/wpcom/stats_country_views/country-views-03-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsCountryViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with null values");

        assert_eq!(response.date, "2026-01-29");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.views.len(), 3);

        // First entry: all nullable fields are null
        let all_nulls = &summary.views[0];
        assert!(all_nulls.location.is_none());
        assert!(all_nulls.views.is_none());
        assert!(all_nulls.country_code.is_none());

        // Second entry: all fields have values
        let all_values = &summary.views[1];
        assert_eq!(all_values.location, Some("United States".to_string()));
        assert_eq!(all_values.views, Some(100));
        assert_eq!(all_values.country_code, Some("US".to_string()));

        // Third entry: mixed null and non-null values
        let partial_nulls = &summary.views[2];
        assert!(partial_nulls.location.is_none());
        assert_eq!(partial_nulls.views, Some(50));
        assert_eq!(partial_nulls.country_code, Some("XX".to_string()));

        // Verify country info with nulls
        let country_info = response
            .country_info
            .as_ref()
            .expect("Country info should be present");

        // Entry with all nulls
        let null_info = country_info
            .get("XX")
            .expect("XX info should exist with nulls");
        assert!(null_info.flag_icon.is_none());
        assert!(null_info.flat_flag_icon.is_none());
        assert!(null_info.country_full.is_none());
        assert!(null_info.map_region.is_none());

        // Entry with all values
        let us_info = country_info.get("US").expect("US info should exist");
        assert!(us_info.flag_icon.is_some());
        assert!(us_info.flat_flag_icon.is_some());
        assert_eq!(us_info.country_full, Some("United States".to_string()));
        assert_eq!(us_info.map_region, Some("021".to_string()));
    }

    #[test]
    fn test_stats_country_views_empty_response() {
        let json_file_path = "tests/wpcom/stats_country_views/country-views-04-empty.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsCountryViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with empty response");

        assert_eq!(response.date, "2026-01-29");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert!(summary.views.is_empty());
        assert_eq!(summary.other_views, 0);
        assert_eq!(summary.total_views, 0);

        // country_info can be null or empty
        assert!(
            response.country_info.is_none() || response.country_info.as_ref().unwrap().is_empty()
        );
    }
}
