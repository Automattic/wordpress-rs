use crate::{
    date::WpDateString,
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The time period for grouping file downloads.
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
pub enum StatsFileDownloadsPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsFileDownloadsPeriod);

/// Parameters for the stats file downloads endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsFileDownloadsParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsFileDownloadsPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<WpDateString>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<WpDateString>,
    /// The maximum number of file downloads to return.
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

impl Default for StatsFileDownloadsParams {
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

impl AppendUrlQueryPairs for StatsFileDownloadsParams {
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

/// Response from the stats file downloads endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains both `summary` and `days` fields
/// - When `summarize=0`: Contains only `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsFileDownloadsResponse {
    /// The date for the stats query.
    pub date: WpDateString,
    /// The time period used for grouping.
    pub period: Option<String>,
    /// Summary data with aggregated file download entries (present when summarize=1).
    pub summary: Option<StatsFileDownloadsSummaryData>,
    /// Per-day stats data keyed by date string.
    pub days: Option<HashMap<String, StatsFileDownloadsDayData>>,
}

/// Summary data with aggregated file download entries.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsFileDownloadsSummaryData {
    /// The list of file download entries.
    pub files: Vec<StatsFileDownloadsEntry>,
    /// Downloads from other files not included in the list.
    pub other_downloads: u64,
    /// The total number of downloads.
    pub total_downloads: u64,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsFileDownloadsDayData {
    /// The list of file download entries for this day.
    pub files: Vec<StatsFileDownloadsEntry>,
    /// Downloads from other files not included in the list.
    pub other_downloads: u64,
    /// The total number of downloads for this day.
    pub total_downloads: u64,
}

/// A file download entry in the stats file downloads response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsFileDownloadsEntry {
    /// The filename of the downloaded file.
    pub filename: Option<String>,
    /// The relative URL path of the file.
    pub relative_url: Option<String>,
    /// The full download URL of the file.
    pub download_url: Option<String>,
    /// The number of downloads.
    pub downloads: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_file_downloads_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/file-downloads",
        )
        .expect("Failed to parse url");

        let params = StatsFileDownloadsParams {
            period: Some(StatsFileDownloadsPeriod::Day),
            date: Some(WpDateString("2026-02-18".to_string())),
            start_date: Some(WpDateString("2026-02-18".to_string())),
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
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/file-downloads?period=day&date=2026-02-18&start_date=2026-02-18&max=10&num=30&locale=en&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_file_downloads_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/file-downloads",
        )
        .expect("Failed to parse url");

        let params = StatsFileDownloadsParams {
            period: Some(StatsFileDownloadsPeriod::Week),
            date: Some(WpDateString("2026-02-18".to_string())),
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
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/file-downloads?period=week&date=2026-02-18&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_file_downloads_params_with_false_values() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/file-downloads",
        )
        .expect("Failed to parse url");

        let params = StatsFileDownloadsParams {
            period: Some(StatsFileDownloadsPeriod::Day),
            summarize: false,
            skip_archives: false,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/file-downloads?period=day&summarize=0&skip_archives=0"
        );
    }

    /// Tests deserialization of all stats file downloads JSON fixtures.
    ///
    /// The `expect_summary` parameter indicates whether the response uses summarize=1
    /// (has `summary` and `days` fields) or summarize=0 (has only `days` field).
    #[rstest]
    #[case("tests/wpcom/stats_file_downloads/summarized-01-day.json", true)]
    #[case("tests/wpcom/stats_file_downloads/no-summary-01.json", false)]
    #[case(
        "tests/wpcom/stats_file_downloads/summarized-02-day-with-nulls.json",
        true
    )]
    #[case(
        "tests/wpcom/stats_file_downloads/summarized-03-day-empty-response.json",
        true
    )]
    fn test_stats_file_downloads_response_deserialization(
        #[case] json_file_path: &str,
        #[case] expect_summary: bool,
    ) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsFileDownloadsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        // Common assertion: date is always present
        assert!(!response.date.0.is_empty());

        if expect_summary {
            // summarize=1 response: has period and summary
            assert!(
                response.period.is_some(),
                "Expected period for summarized response"
            );
            assert!(!response.period.as_ref().unwrap().is_empty());

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
    fn test_stats_file_downloads_response_deserialization_summary() {
        let json_file_path = "tests/wpcom/stats_file_downloads/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsFileDownloadsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-02-18");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_downloads, 83);
        assert_eq!(summary.other_downloads, 0);
        assert_eq!(summary.files.len(), 5);

        // Verify first file entry
        let first_file = &summary.files[0];
        assert_eq!(first_file.filename, Some("automattic-w-9.pdf".to_string()));
        assert_eq!(
            first_file.relative_url,
            Some("/2025/01/automattic-w-9.pdf".to_string())
        );
        assert_eq!(first_file.downloads, Some(5));
    }

    #[test]
    fn test_stats_file_downloads_response_deserialization_days() {
        let json_file_path = "tests/wpcom/stats_file_downloads/no-summary-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsFileDownloadsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-02-18");
        assert!(response.summary.is_none());

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 3);

        // Verify day with files
        let day = days.get("2026-02-18").expect("2026-02-18 should exist");
        assert_eq!(day.total_downloads, 34);
        assert_eq!(day.other_downloads, 0);
        assert_eq!(day.files.len(), 3);

        let file_entry = &day.files[0];
        assert_eq!(file_entry.filename, Some("automattic-w-9.pdf".to_string()));
        assert_eq!(file_entry.downloads, Some(4));

        // Verify empty day
        let empty_day = days.get("2026-02-14").expect("2026-02-14 should exist");
        assert_eq!(empty_day.total_downloads, 0);
        assert!(empty_day.files.is_empty());
    }

    #[test]
    fn test_stats_file_downloads_with_null_values() {
        let json_file_path = "tests/wpcom/stats_file_downloads/summarized-02-day-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsFileDownloadsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with null values");

        assert_eq!(response.date.0, "2026-02-18");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_downloads, 10);
        assert_eq!(summary.files.len(), 2);

        // First entry: all nullable fields are null
        let all_nulls = &summary.files[0];
        assert!(all_nulls.filename.is_none());
        assert!(all_nulls.relative_url.is_none());
        assert!(all_nulls.download_url.is_none());
        assert!(all_nulls.downloads.is_none());

        // Second entry: has values
        let with_values = &summary.files[1];
        assert_eq!(with_values.filename, Some("example-file.pdf".to_string()));
        assert_eq!(
            with_values.relative_url,
            Some("/2026/02/example-file.pdf".to_string())
        );
        assert_eq!(with_values.downloads, Some(10));
    }
}
