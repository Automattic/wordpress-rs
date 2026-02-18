use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The time period for grouping clicks.
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
pub enum StatsClicksPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsClicksPeriod);

/// Parameters for the stats clicks endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsClicksParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsClicksPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The maximum number of clicks to return.
    #[uniffi(default = None)]
    pub max: Option<u32>,
    /// The number of periods to include in the response.
    #[uniffi(default = None)]
    pub num: Option<u32>,
    /// The locale for the response.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
    /// Whether to return a summary of the data.
    ///
    /// - `true` (default): Response contains `summary` field with aggregated data
    /// - `false`: Response contains `days` field with per-day breakdown
    #[uniffi(default = true)]
    pub summarize: bool,
    /// Whether to skip archive pages (date-based archives, category archives, etc.) in the response.
    ///
    /// - `Some(true)` (default): Archive pages are excluded from results
    /// - `Some(false)`: Archive pages are included in results
    /// - `None`: Parameter is not sent to the API
    #[uniffi(default = Some(true))]
    pub skip_archives: Option<bool>,
}

impl Default for StatsClicksParams {
    fn default() -> Self {
        Self {
            period: None,
            date: None,
            start_date: None,
            max: None,
            num: None,
            locale: None,
            summarize: true,
            skip_archives: Some(true),
        }
    }
}

impl AppendUrlQueryPairs for StatsClicksParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_query_value_pair("summarize", &(self.summarize as u32))
            .append_option_query_value_pair(
                "skip_archives",
                self.skip_archives.map(|b| b as u32).as_ref(),
            );
    }
}

/// Response from the stats clicks endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize` is not set: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsClicksResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time period used for grouping (present when summarize=1).
    pub period: Option<String>,
    /// Summary data with aggregated click groups (present when summarize=1).
    pub summary: Option<StatsClicksSummaryData>,
    /// Per-day stats data keyed by date string (present when summarize is not set).
    pub days: Option<HashMap<String, StatsClicksDayData>>,
}

/// Summary data with aggregated click groups.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsClicksSummaryData {
    /// The list of click entries.
    pub clicks: Vec<StatsClicksEntry>,
    /// Clicks from other sources not included in the list.
    pub other_clicks: u64,
    /// The total number of clicks.
    pub total_clicks: u64,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsClicksDayData {
    /// The list of click entries for this day.
    pub clicks: Vec<StatsClicksEntry>,
    /// Clicks from other sources not included in the list.
    pub other_clicks: u64,
    /// The total number of clicks for this day.
    pub total_clicks: u64,
}

/// A click entry in the stats clicks response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsClicksEntry {
    /// The icon URL for the click source.
    pub icon: Option<String>,
    /// The URL of the click source.
    pub url: Option<String>,
    /// The display name of the click source.
    pub name: Option<String>,
    /// The number of views/clicks.
    pub views: Option<u64>,
    /// Child click entries (present when the source groups multiple URLs).
    pub children: Option<Vec<StatsClicksChild>>,
}

/// A child click entry within a click group.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsClicksChild {
    /// The URL of the child click.
    pub url: Option<String>,
    /// The display name of the child click.
    pub name: Option<String>,
    /// The number of views/clicks for this child.
    pub views: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_clicks_params_serialization() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks")
                .expect("Failed to parse url");

        let params = StatsClicksParams {
            period: Some(StatsClicksPeriod::Day),
            date: Some("2026-02-18".to_string()),
            start_date: Some("2026-02-18".to_string()),
            max: Some(10),
            num: Some(30),
            locale: Some(WPComLanguage::English),
            summarize: true,
            skip_archives: Some(true),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks?period=day&date=2026-02-18&start_date=2026-02-18&max=10&num=30&locale=en&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_clicks_params_serialization_partial() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks")
                .expect("Failed to parse url");

        let params = StatsClicksParams {
            period: Some(StatsClicksPeriod::Week),
            date: Some("2026-02-18".to_string()),
            start_date: None,
            max: None,
            num: None,
            locale: None,
            summarize: true,
            skip_archives: Some(true),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks?period=week&date=2026-02-18&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_clicks_params_without_summarize_and_skip_archives() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks")
                .expect("Failed to parse url");

        let params = StatsClicksParams {
            period: Some(StatsClicksPeriod::Day),
            date: Some("2026-02-18".to_string()),
            summarize: false,
            skip_archives: None,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks?period=day&date=2026-02-18&summarize=0"
        );
    }

    #[test]
    fn test_stats_clicks_params_with_false_values() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks")
                .expect("Failed to parse url");

        let params = StatsClicksParams {
            period: Some(StatsClicksPeriod::Day),
            summarize: false,
            skip_archives: Some(false),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/clicks?period=day&summarize=0&skip_archives=0"
        );
    }

    /// Tests deserialization of all stats clicks JSON fixtures.
    ///
    /// The `expect_summary` parameter indicates whether the response uses summarize=1
    /// (has `summary` and `period` fields) or summarize=0 (has `days` field instead).
    #[rstest]
    #[case("tests/wpcom/stats_clicks/summarized-01-day.json", true)]
    #[case("tests/wpcom/stats_clicks/no-summary-01.json", false)]
    #[case("tests/wpcom/stats_clicks/summarized-02-day-with-nulls.json", true)]
    #[case("tests/wpcom/stats_clicks/summarized-03-day-empty-response.json", true)]
    fn test_stats_clicks_response_deserialization(
        #[case] json_file_path: &str,
        #[case] expect_summary: bool,
    ) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsClicksResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        // Common assertion: date is always present
        assert!(!response.date.is_empty());

        if expect_summary {
            // summarize=1 response: has period and summary, no days
            assert!(
                response.period.is_some(),
                "Expected period for summarized response"
            );
            assert!(!response.period.as_ref().unwrap().is_empty());
            assert!(
                response.days.is_none(),
                "Summarized response should not have days"
            );

            response
                .summary
                .as_ref()
                .expect("Summary should be present for summarized response");
        } else {
            // summarize=0 response: has days, no summary
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
    fn test_stats_clicks_response_deserialization_summary() {
        let json_file_path = "tests/wpcom/stats_clicks/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsClicksResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-18");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_clicks, 20);
        assert_eq!(summary.other_clicks, 0);
        assert_eq!(summary.clicks.len(), 2);

        // Verify first click entry (with children)
        let first_click = &summary.clicks[0];
        assert_eq!(first_click.name, Some("href.li".to_string()));
        assert!(first_click.url.is_none());
        assert_eq!(first_click.views, Some(17));

        let children = first_click
            .children
            .as_ref()
            .expect("Children should be present");
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].views, Some(2));
    }

    #[test]
    fn test_stats_clicks_response_deserialization_days() {
        let json_file_path = "tests/wpcom/stats_clicks/no-summary-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsClicksResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-18");
        assert!(response.summary.is_none());

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 2);

        // Verify day with clicks
        let day = days.get("2026-02-11").expect("2026-02-11 should exist");
        assert_eq!(day.total_clicks, 1);
        assert_eq!(day.other_clicks, 0);
        assert_eq!(day.clicks.len(), 1);

        let click = &day.clicks[0];
        assert_eq!(click.name, Some("example.com/page".to_string()));
        assert_eq!(click.views, Some(1));
        assert!(click.children.is_none());

        // Verify empty day
        let empty_day = days.get("2026-02-18").expect("2026-02-18 should exist");
        assert_eq!(empty_day.total_clicks, 0);
        assert!(empty_day.clicks.is_empty());
    }

    #[test]
    fn test_stats_clicks_with_null_values() {
        let json_file_path = "tests/wpcom/stats_clicks/summarized-02-day-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsClicksResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with null values");

        assert_eq!(response.date, "2026-02-18");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_clicks, 5);
        assert_eq!(summary.clicks.len(), 2);

        // First entry: all nullable fields are null
        let all_nulls = &summary.clicks[0];
        assert!(all_nulls.icon.is_none());
        assert!(all_nulls.url.is_none());
        assert!(all_nulls.name.is_none());
        assert!(all_nulls.views.is_none());
        assert!(all_nulls.children.is_none());

        // Second entry: has children with null values
        let with_children = &summary.clicks[1];
        assert_eq!(with_children.name, Some("example.com".to_string()));
        assert_eq!(with_children.views, Some(5));

        let children = with_children
            .children
            .as_ref()
            .expect("Children should be present");
        assert_eq!(children.len(), 2);

        // First child: all nulls
        assert!(children[0].url.is_none());
        assert!(children[0].name.is_none());
        assert!(children[0].views.is_none());

        // Second child: all values
        assert_eq!(
            children[1].url,
            Some("https://example.com/page".to_string())
        );
        assert_eq!(children[1].name, Some("example.com/page".to_string()));
        assert_eq!(children[1].views, Some(3));
    }
}
