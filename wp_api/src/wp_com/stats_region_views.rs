use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::deserialize_option_empty_array_or_hashmap;

/// The time period for grouping region views.
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
pub enum StatsRegionViewsPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsRegionViewsPeriod);

/// Parameters for the stats region views endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsRegionViewsParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsRegionViewsPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The maximum number of regions to return.
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
    /// - `true` (default): Response contains `summary` field with aggregated region
    ///   views across all requested periods. The `days` field will be absent.
    /// - `false`: Response contains `days` field with a per-day
    ///   breakdown of region views, where each day is keyed by its date string. The `summary`
    ///   field will be absent.
    #[uniffi(default = true)]
    pub summarize: bool,
}

impl Default for StatsRegionViewsParams {
    fn default() -> Self {
        Self {
            period: None,
            date: None,
            start_date: None,
            max: None,
            num: None,
            days: None,
            locale: None,
            summarize: true,
        }
    }
}

impl AppendUrlQueryPairs for StatsRegionViewsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_option_query_value_pair("days", self.days.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_query_value_pair("summarize", &(self.summarize as u32));
    }
}

/// Response from the stats region views endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize` is not set: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsRegionViewsResponse {
    /// The date for the stats query.
    pub date: String,
    /// Country information keyed by country code.
    /// Can be `null`, an empty array `[]`, or a map of country codes to info.
    #[serde(
        rename = "country-info",
        deserialize_with = "deserialize_option_empty_array_or_hashmap"
    )]
    pub country_info: Option<HashMap<String, StatsRegionCountryInfo>>,
    /// Summary data with aggregated region views (present when summarize=1).
    pub summary: Option<StatsRegionViewsSummaryData>,
    /// Per-day stats data keyed by date string (present when summarize is not set).
    pub days: Option<HashMap<String, StatsRegionViewsDayData>>,
}

/// Country information including flag icons and display name.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsRegionCountryInfo {
    /// The URL of the flag icon.
    pub flag_icon: Option<String>,
    /// The URL of the flat flag icon.
    pub flat_flag_icon: Option<String>,
    /// The full country name.
    pub country_full: Option<String>,
    /// The map region code.
    pub map_region: Option<String>,
}

/// Summary data with aggregated region views.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsRegionViewsSummaryData {
    /// The list of region views.
    pub views: Vec<StatsRegionView>,
    /// Views from other regions not included in the list.
    pub other_views: u64,
    /// The total number of views.
    pub total_views: u64,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsRegionViewsDayData {
    /// The list of region views for this day.
    pub views: Vec<StatsRegionView>,
    /// Views from other regions not included in the list.
    pub other_views: u64,
    /// The total number of views for this day.
    pub total_views: u64,
}

/// A region view entry in the stats region views response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsRegionView {
    /// The region/state/province name.
    pub location: Option<String>,
    /// The number of views from this region.
    pub views: Option<u64>,
    /// The country code.
    pub country_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_region_views_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/location-views/region",
        )
        .expect("Failed to parse url");

        let params = StatsRegionViewsParams {
            period: Some(StatsRegionViewsPeriod::Day),
            date: Some("2026-02-05".to_string()),
            start_date: Some("2026-01-30".to_string()),
            max: Some(10),
            num: Some(1),
            days: Some(1),
            locale: Some(WPComLanguage::English),
            summarize: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/location-views/region?period=day&date=2026-02-05&start_date=2026-01-30&max=10&num=1&days=1&locale=en&summarize=1"
        );
    }

    #[test]
    fn test_stats_region_views_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/location-views/region",
        )
        .expect("Failed to parse url");

        let params = StatsRegionViewsParams {
            period: Some(StatsRegionViewsPeriod::Day),
            date: Some("2026-02-05".to_string()),
            start_date: Some("2026-01-30".to_string()),
            locale: Some(WPComLanguage::English),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/location-views/region?period=day&date=2026-02-05&start_date=2026-01-30&locale=en&summarize=1"
        );
    }

    #[test]
    fn test_stats_region_views_params_with_false_summarize() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/location-views/region",
        )
        .expect("Failed to parse url");

        let params = StatsRegionViewsParams {
            period: Some(StatsRegionViewsPeriod::Day),
            summarize: false,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/location-views/region?period=day&summarize=0"
        );
    }

    /// Tests deserialization of all stats region views JSON fixtures.
    #[rstest]
    #[case("tests/wpcom/stats_region_views/summarized-01-day.json", true)]
    #[case("tests/wpcom/stats_region_views/no-summary-01.json", false)]
    fn test_stats_region_views_response_deserialization(
        #[case] json_file_path: &str,
        #[case] expect_summary: bool,
    ) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsRegionViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());

        if expect_summary {
            assert!(
                response.days.is_none(),
                "Summarized response should not have days"
            );

            response
                .summary
                .as_ref()
                .expect("Summary should be present for summarized response");
        } else {
            assert!(
                response.summary.is_none(),
                "Days response should not have summary"
            );

            let days = response
                .days
                .as_ref()
                .expect("Days should be present for non-summarized response");
            assert!(!days.is_empty());
        }
    }

    #[test]
    fn test_stats_region_views_response_deserialization_summary_01() {
        let json_file_path = "tests/wpcom/stats_region_views/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsRegionViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-05");
        assert!(response.summary.is_some());
        assert!(response.days.is_none());

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert!(!summary.views.is_empty());

        // Verify first region view
        let first_view = &summary.views[0];
        assert_eq!(first_view.location, Some("Washington".to_string()));
        assert_eq!(first_view.views, Some(613));
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
    fn test_stats_region_views_response_deserialization_no_summary_01() {
        let json_file_path = "tests/wpcom/stats_region_views/no-summary-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsRegionViewsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-05");
        assert!(response.summary.is_none());
        assert!(response.days.is_some());

        let days = response.days.as_ref().expect("Days should be present");
        assert!(!days.is_empty());

        // Verify first day
        let day1 = days.get("2026-02-05").expect("2026-02-05 should exist");
        assert_eq!(day1.total_views, 504);
        assert_eq!(day1.other_views, 157);
        assert!(!day1.views.is_empty());

        // Verify first region view of first day
        let first_view = &day1.views[0];
        assert_eq!(first_view.location, Some("Washington".to_string()));
        assert_eq!(first_view.views, Some(73));
        assert_eq!(first_view.country_code, Some("US".to_string()));

        // Verify country info
        let country_info = response
            .country_info
            .as_ref()
            .expect("Country info should be present");
        assert!(country_info.contains_key("US"));
        assert!(country_info.contains_key("ES"));
    }
}
