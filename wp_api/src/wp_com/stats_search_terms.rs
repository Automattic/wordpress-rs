use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The time period for grouping search terms.
#[derive(
    Debug,
    Default,
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
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum StatsSearchTermsPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsSearchTermsPeriod);

/// Parameters for the stats search terms endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsSearchTermsParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsSearchTermsPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The maximum number of search terms to return.
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
    /// - `true` (default): Archive pages are excluded from results
    /// - `false`: Archive pages are included in results
    #[uniffi(default = true)]
    pub skip_archives: bool,
}

impl Default for StatsSearchTermsParams {
    fn default() -> Self {
        Self {
            period: None,
            date: None,
            start_date: None,
            max: None,
            num: None,
            locale: None,
            summarize: true,
            skip_archives: true,
        }
    }
}

impl AppendUrlQueryPairs for StatsSearchTermsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_query_value_pair("summarize", &(self.summarize as u32))
            .append_query_value_pair("skip_archives", &(self.skip_archives as u32));
    }
}

/// Response from the stats search terms endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize` is not set: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSearchTermsResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time period used for grouping (present when summarize=1).
    pub period: Option<String>,
    /// Summary data with aggregated search terms (present when summarize=1).
    pub summary: Option<StatsSearchTermsSummaryData>,
    /// Per-day stats data keyed by date string (present when summarize is not set).
    pub days: Option<HashMap<String, StatsSearchTermsDayData>>,
}

/// Summary data with aggregated search terms.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSearchTermsSummaryData {
    /// The list of search term entries.
    pub search_terms: Vec<StatsSearchTermsEntry>,
    /// The number of encrypted search terms.
    pub encrypted_search_terms: i64,
    /// The number of other search terms not included in the list.
    pub other_search_terms: i64,
    /// The total number of search terms.
    pub total_search_terms: i64,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSearchTermsDayData {
    /// The list of search term entries for this day.
    pub search_terms: Vec<StatsSearchTermsEntry>,
    /// The number of encrypted search terms for this day.
    pub encrypted_search_terms: i64,
    /// The number of other search terms not included in the list for this day.
    pub other_search_terms: i64,
    /// The total number of search terms for this day.
    pub total_search_terms: i64,
}

/// A search term entry in the stats search terms response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSearchTermsEntry {
    /// The search term.
    pub term: Option<String>,
    /// The number of views from this search term.
    pub views: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_search_terms_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/search-terms",
        )
        .expect("Failed to parse url");

        let params = StatsSearchTermsParams {
            period: Some(StatsSearchTermsPeriod::Day),
            date: Some("2026-02-18".to_string()),
            start_date: Some("2026-02-18".to_string()),
            max: Some(10),
            num: Some(30),
            locale: Some(WPComLanguage::English),
            summarize: true,
            skip_archives: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/search-terms?period=day&date=2026-02-18&start_date=2026-02-18&max=10&num=30&locale=en&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_search_terms_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/search-terms",
        )
        .expect("Failed to parse url");

        let params = StatsSearchTermsParams {
            period: Some(StatsSearchTermsPeriod::Week),
            date: Some("2026-02-18".to_string()),
            start_date: None,
            max: None,
            num: None,
            locale: None,
            summarize: true,
            skip_archives: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/search-terms?period=week&date=2026-02-18&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_search_terms_params_with_false_values() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/search-terms",
        )
        .expect("Failed to parse url");

        let params = StatsSearchTermsParams {
            period: Some(StatsSearchTermsPeriod::Day),
            summarize: false,
            skip_archives: false,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/search-terms?period=day&summarize=0&skip_archives=0"
        );
    }

    /// Tests deserialization of all stats search terms JSON fixtures.
    ///
    /// The `expect_summary` parameter indicates whether the response uses summarize=1
    /// (has `summary` and `period` fields) or summarize=0 (has `days` field instead).
    #[rstest]
    #[case("tests/wpcom/stats_search_terms/summarized-01-day.json", true)]
    #[case("tests/wpcom/stats_search_terms/no-summary-01.json", false)]
    #[case(
        "tests/wpcom/stats_search_terms/summarized-02-day-with-nulls.json",
        true
    )]
    #[case(
        "tests/wpcom/stats_search_terms/summarized-03-day-empty-response.json",
        true
    )]
    fn test_stats_search_terms_response_deserialization(
        #[case] json_file_path: &str,
        #[case] expect_summary: bool,
    ) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSearchTermsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        // Common assertion: date is always present
        assert!(!response.date.is_empty());

        if expect_summary {
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
    fn test_stats_search_terms_response_deserialization_summary() {
        let json_file_path = "tests/wpcom/stats_search_terms/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSearchTermsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-18");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_search_terms, 0);
        assert_eq!(summary.encrypted_search_terms, 14);
        assert_eq!(summary.other_search_terms, -429);
        assert_eq!(summary.search_terms.len(), 3);

        // Verify first search term
        let first_term = &summary.search_terms[0];
        assert_eq!(
            first_term.term,
            Some("https://example.com/support-tools/".to_string())
        );
        assert_eq!(first_term.views, Some(12));
    }

    #[test]
    fn test_stats_search_terms_response_deserialization_days() {
        let json_file_path = "tests/wpcom/stats_search_terms/no-summary-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSearchTermsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-18");
        assert!(response.summary.is_none());

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 2);

        // Verify day with search terms
        let day = days.get("2026-02-17").expect("2026-02-17 should exist");
        assert_eq!(day.total_search_terms, 8);
        assert_eq!(day.encrypted_search_terms, 2);
        assert_eq!(day.other_search_terms, 1);
        assert_eq!(day.search_terms.len(), 1);
        assert_eq!(day.search_terms[0].term, Some("example search".to_string()));
        assert_eq!(day.search_terms[0].views, Some(5));

        // Verify empty day
        let empty_day = days.get("2026-02-18").expect("2026-02-18 should exist");
        assert_eq!(empty_day.total_search_terms, 0);
        assert!(empty_day.search_terms.is_empty());
    }

    #[test]
    fn test_stats_search_terms_with_null_values() {
        let json_file_path = "tests/wpcom/stats_search_terms/summarized-02-day-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSearchTermsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with null values");

        assert_eq!(response.date, "2026-02-18");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_search_terms, 15);
        assert_eq!(summary.search_terms.len(), 2);

        // First entry: all nullable fields are null
        let all_nulls = &summary.search_terms[0];
        assert!(all_nulls.term.is_none());
        assert!(all_nulls.views.is_none());

        // Second entry: all fields have values
        let with_values = &summary.search_terms[1];
        assert_eq!(with_values.term, Some("example query".to_string()));
        assert_eq!(with_values.views, Some(10));
    }
}
