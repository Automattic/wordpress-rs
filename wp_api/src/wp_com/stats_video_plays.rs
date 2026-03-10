use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};

/// The time period for grouping video plays.
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
pub enum StatsVideoPlaysPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsVideoPlaysPeriod);

/// Parameters for the stats video plays endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsVideoPlaysParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsVideoPlaysPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The maximum number of video plays to return.
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
    /// - `true` (default): Response contains summarized data
    /// - `false`: Response contains per-day breakdown
    #[uniffi(default = true)]
    pub summarize: bool,
    /// Whether to include complete stats (including zero-view videos).
    ///
    /// - `Some(true)` (default): Include all videos
    /// - `Some(false)`: Only include videos with views
    /// - `None`: Parameter is not sent to the API
    #[uniffi(default = Some(true))]
    pub complete_stats: Option<bool>,
}

impl Default for StatsVideoPlaysParams {
    fn default() -> Self {
        Self {
            period: None,
            date: None,
            start_date: None,
            max: None,
            num: None,
            locale: None,
            summarize: true,
            complete_stats: Some(true),
        }
    }
}

impl AppendUrlQueryPairs for StatsVideoPlaysParams {
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
                "complete_stats",
                self.complete_stats.map(|b| b as u32).as_ref(),
            );
    }
}

/// Response from the stats video plays endpoint.
///
/// The video plays endpoint always returns data inside `days.summary`,
/// regardless of the `summarize` parameter.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVideoPlaysResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time period used for grouping.
    pub period: Option<String>,
    /// The days data containing the summary.
    pub days: StatsVideoPlaysDays,
}

/// The days wrapper containing the summary data.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVideoPlaysDays {
    /// The summary data with video play entries and totals.
    pub summary: StatsVideoPlaysSummaryData,
}

/// Summary data with video play entries and totals.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVideoPlaysSummaryData {
    /// The list of video play entries.
    pub data: Vec<StatsVideoPlaysEntry>,
    /// The total aggregated stats.
    pub total: StatsVideoPlaysTotal,
}

/// A video play entry in the stats video plays response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVideoPlaysEntry {
    /// The post ID of the video.
    pub post_id: u64,
    /// The title of the video.
    pub title: Option<String>,
    /// The number of views/plays.
    pub views: Option<u64>,
    /// The number of impressions.
    pub impressions: Option<u64>,
    /// The total watch time in hours.
    pub watch_time: Option<f64>,
    /// The retention rate percentage.
    pub retention_rate: Option<f64>,
}

/// Total aggregated video play stats.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsVideoPlaysTotal {
    /// The total number of impressions.
    pub impressions: u64,
    /// The total number of views/plays.
    pub views: u64,
    /// The total watch time in hours.
    pub watch_time: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_video_plays_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays",
        )
        .expect("Failed to parse url");

        let params = StatsVideoPlaysParams {
            period: Some(StatsVideoPlaysPeriod::Day),
            date: Some("2026-02-18".to_string()),
            start_date: Some("2026-02-12".to_string()),
            max: Some(10),
            num: Some(30),
            locale: Some(WPComLanguage::English),
            summarize: true,
            complete_stats: Some(true),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays?period=day&date=2026-02-18&start_date=2026-02-12&max=10&num=30&locale=en&summarize=1&complete_stats=1"
        );
    }

    #[test]
    fn test_stats_video_plays_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays",
        )
        .expect("Failed to parse url");

        let params = StatsVideoPlaysParams {
            period: Some(StatsVideoPlaysPeriod::Week),
            date: Some("2026-02-18".to_string()),
            start_date: None,
            max: None,
            num: None,
            locale: None,
            summarize: true,
            complete_stats: Some(true),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays?period=week&date=2026-02-18&summarize=1&complete_stats=1"
        );
    }

    #[test]
    fn test_stats_video_plays_params_without_summarize_and_complete_stats() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays",
        )
        .expect("Failed to parse url");

        let params = StatsVideoPlaysParams {
            period: Some(StatsVideoPlaysPeriod::Day),
            date: Some("2026-02-18".to_string()),
            summarize: false,
            complete_stats: None,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays?period=day&date=2026-02-18&summarize=0"
        );
    }

    #[test]
    fn test_stats_video_plays_params_with_false_values() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays",
        )
        .expect("Failed to parse url");

        let params = StatsVideoPlaysParams {
            period: Some(StatsVideoPlaysPeriod::Day),
            summarize: false,
            complete_stats: Some(false),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/video-plays?period=day&summarize=0&complete_stats=0"
        );
    }

    /// Tests deserialization of all stats video plays JSON fixtures.
    #[rstest]
    #[case("tests/wpcom/stats_video_plays/summarized-01-day.json")]
    #[case("tests/wpcom/stats_video_plays/no-summary-01.json")]
    #[case("tests/wpcom/stats_video_plays/summarized-02-day-with-nulls.json")]
    #[case("tests/wpcom/stats_video_plays/summarized-03-day-empty-response.json")]
    fn test_stats_video_plays_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVideoPlaysResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        // Common assertion: date is always present
        assert!(!response.date.is_empty());
        assert!(response.period.is_some());
    }

    #[test]
    fn test_stats_video_plays_response_deserialization_summary() {
        let json_file_path = "tests/wpcom/stats_video_plays/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVideoPlaysResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-18");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = &response.days.summary;
        assert_eq!(summary.data.len(), 3);
        assert_eq!(summary.total.impressions, 1172);
        assert_eq!(summary.total.views, 31);
        assert!((summary.total.watch_time - 0.6936111111111113).abs() < f64::EPSILON);

        // Verify first video play entry
        let first_entry = &summary.data[0];
        assert_eq!(first_entry.post_id, 282653);
        assert_eq!(first_entry.title, Some("example_video_1".to_string()));
        assert_eq!(first_entry.views, Some(10));
        assert_eq!(first_entry.impressions, Some(71));
    }

    #[test]
    fn test_stats_video_plays_with_null_values() {
        let json_file_path = "tests/wpcom/stats_video_plays/summarized-02-day-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVideoPlaysResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with null values");

        assert_eq!(response.date, "2026-02-18");

        let summary = &response.days.summary;
        assert_eq!(summary.data.len(), 2);

        // First entry: nullable fields are null
        let null_entry = &summary.data[0];
        assert_eq!(null_entry.post_id, 0);
        assert!(null_entry.title.is_none());
        assert!(null_entry.views.is_none());
        assert!(null_entry.impressions.is_none());
        assert!(null_entry.watch_time.is_none());
        assert!(null_entry.retention_rate.is_none());

        // Second entry: all fields have values
        let valid_entry = &summary.data[1];
        assert_eq!(valid_entry.post_id, 282653);
        assert_eq!(valid_entry.title, Some("example_video".to_string()));
        assert_eq!(valid_entry.views, Some(10));
        assert_eq!(valid_entry.impressions, Some(71));
    }

    #[test]
    fn test_stats_video_plays_empty_response() {
        let json_file_path = "tests/wpcom/stats_video_plays/summarized-03-day-empty-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsVideoPlaysResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let summary = &response.days.summary;
        assert!(summary.data.is_empty());
        assert_eq!(summary.total.impressions, 0);
        assert_eq!(summary.total.views, 0);
        assert!((summary.total.watch_time - 0.0).abs() < f64::EPSILON);
    }
}
