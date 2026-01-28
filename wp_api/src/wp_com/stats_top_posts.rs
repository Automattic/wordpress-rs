use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The time period for grouping top posts.
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
pub enum StatsTopPostsPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsTopPostsPeriod);

/// Parameters for the stats top posts endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsTopPostsParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsTopPostsPeriod>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The maximum number of top posts to return.
    #[uniffi(default = None)]
    pub max: Option<u32>,
    /// The number of periods to include in the response.
    #[uniffi(default = None)]
    pub num: Option<u32>,
    /// The locale for the response.
    #[uniffi(default = None)]
    pub locale: Option<String>,
    /// Whether to return a summary of the data.
    ///
    /// - `Some(true)` (default): Response contains `summary` field with aggregated data
    /// - `Some(false)`: Response contains `days` field with per-day breakdown
    /// - `None`: Parameter is not sent to the API
    #[uniffi(default = Some(true))]
    pub summarize: Option<bool>,
    /// Whether to skip archive pages (date-based archives, category archives, etc.) in the response.
    ///
    /// - `Some(true)` (default): Archive pages are excluded from results
    /// - `Some(false)`: Archive pages are included in results
    /// - `None`: Parameter is not sent to the API
    #[uniffi(default = Some(true))]
    pub skip_archives: Option<bool>,
}

impl AppendUrlQueryPairs for StatsTopPostsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_option_query_value_pair("summarize", self.summarize.map(|b| b as u32).as_ref())
            .append_option_query_value_pair(
                "skip_archives",
                self.skip_archives.map(|b| b as u32).as_ref(),
            );
    }
}

/// Response from the stats top posts endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize` is not set: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopPostsResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time period used for grouping (present when summarize=1).
    pub period: Option<String>,
    /// Summary data with aggregated post views (present when summarize=1).
    pub summary: Option<StatsTopPostsSummaryData>,
    /// Per-day stats data keyed by date string (present when summarize is not set).
    pub days: Option<HashMap<String, StatsTopPostsDayData>>,
}

/// Summary data with aggregated post views.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopPostsSummaryData {
    /// The list of post views.
    pub postviews: Vec<StatsTopPostsPostView>,
    /// The total number of views.
    pub total_views: u64,
    /// IDs that were dropped from the results (may be absent from API response).
    #[serde(default)]
    pub dropped_ids: Vec<u64>,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopPostsDayData {
    /// The list of post views for this day.
    pub postviews: Vec<StatsTopPostsPostView>,
    /// The total number of views for this day.
    pub total_views: u64,
    /// IDs that were dropped from the results (may be absent from API response).
    #[serde(default)]
    pub dropped_ids: Vec<u64>,
    /// Views from other posts not included in the list.
    pub other_views: u64,
}

/// A post view entry in the stats top posts response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopPostsPostView {
    /// The post ID.
    pub id: u64,
    /// The URL of the post (can be null for homepage).
    pub href: Option<String>,
    /// The publication date of the post (can be null for homepage).
    pub date: Option<String>,
    /// The title of the post.
    pub title: Option<String>,
    /// The type of the content (post, page, homepage, etc.).
    #[serde(rename = "type")]
    pub post_type: Option<String>,
    /// The publication status (can be null for homepage).
    pub status: Option<String>,
    /// Whether the post is public.
    pub public: Option<bool>,
    /// The number of views.
    pub views: Option<u64>,
    /// Whether this is a video play.
    pub video_play: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_top_posts_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts",
        )
        .expect("Failed to parse url");

        let params = StatsTopPostsParams {
            period: Some(StatsTopPostsPeriod::Day),
            start_date: Some("2026-01-26".to_string()),
            date: Some("2026-01-26".to_string()),
            max: Some(10),
            num: Some(30),
            locale: Some("en".to_string()),
            summarize: Some(true),
            skip_archives: Some(true),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts?period=day&start_date=2026-01-26&date=2026-01-26&max=10&num=30&locale=en&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_top_posts_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts",
        )
        .expect("Failed to parse url");

        let params = StatsTopPostsParams {
            period: Some(StatsTopPostsPeriod::Week),
            start_date: None,
            date: Some("2026-01-19".to_string()),
            max: None,
            num: None,
            locale: None,
            summarize: Some(true),
            skip_archives: Some(true),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts?period=week&date=2026-01-19&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_top_posts_params_without_summarize_and_skip_archives() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts",
        )
        .expect("Failed to parse url");

        let params = StatsTopPostsParams {
            period: Some(StatsTopPostsPeriod::Day),
            date: Some("2026-01-26".to_string()),
            summarize: None,
            skip_archives: None,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts?period=day&date=2026-01-26"
        );
    }

    #[test]
    fn test_stats_top_posts_params_with_false_values() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts",
        )
        .expect("Failed to parse url");

        let params = StatsTopPostsParams {
            period: Some(StatsTopPostsPeriod::Day),
            summarize: Some(false),
            skip_archives: Some(false),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts?period=day&summarize=0&skip_archives=0"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_top_posts/top-posts-01.json")]
    fn test_stats_top_posts_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(response.period.is_some());
        assert!(!response.period.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_stats_top_posts_response_deserialization_top_posts_01() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-25");
        assert_eq!(response.period, Some("week".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_views, 2996);
        assert!(summary.dropped_ids.is_empty());
        assert_eq!(summary.postviews.len(), 10);

        // Verify first post view
        let first_post = &summary.postviews[0];
        assert_eq!(first_post.id, 269);
        assert_eq!(first_post.title, Some("Welcome to Automattic".to_string()));
        assert_eq!(first_post.post_type, Some("page".to_string()));
        assert_eq!(first_post.status, Some("publish".to_string()));
        assert_eq!(first_post.public, Some(true));
        assert_eq!(first_post.views, Some(417));
        assert_eq!(first_post.video_play, Some(false));
    }

    #[test]
    fn test_stats_top_posts_homepage_entry() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        // Find homepage entry (id: 0, type: homepage, date: null, status: null)
        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        let homepage = summary
            .postviews
            .iter()
            .find(|p| p.post_type == Some("homepage".to_string()))
            .expect("Homepage entry should exist");

        assert_eq!(homepage.id, 0);
        assert_eq!(homepage.title, Some("Home page / Archives".to_string()));
        assert!(homepage.href.is_none());
        assert!(homepage.date.is_none());
        assert!(homepage.status.is_none());
        assert_eq!(homepage.public, Some(false));
        assert_eq!(homepage.views, Some(244));
    }

    #[rstest]
    #[case("tests/wpcom/stats_top_posts/top-posts-02-days.json")]
    fn test_stats_top_posts_response_deserialization_days(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(response.summary.is_none());
        assert!(response.days.is_some());
    }

    #[test]
    fn test_stats_top_posts_response_deserialization_days_02() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-02-days.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-26");
        assert!(response.period.is_none());
        assert!(response.summary.is_none());

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 2);

        // Verify first day
        let day1 = days.get("2026-01-26").expect("2026-01-26 should exist");
        assert_eq!(day1.total_views, 220);
        assert_eq!(day1.other_views, 10);
        assert!(day1.dropped_ids.is_empty());
        assert_eq!(day1.postviews.len(), 3);

        // Verify first post view of first day
        let first_post = &day1.postviews[0];
        assert_eq!(first_post.id, 269);
        assert_eq!(first_post.title, Some("Welcome to Automattic".to_string()));
        assert_eq!(first_post.views, Some(150));

        // Verify second day
        let day2 = days.get("2026-01-25").expect("2026-01-25 should exist");
        assert_eq!(day2.total_views, 180);
        assert_eq!(day2.other_views, 60);
        assert_eq!(day2.dropped_ids, vec![12345]);
        assert_eq!(day2.postviews.len(), 1);
    }

    #[test]
    fn test_stats_top_posts_with_null_values() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-03-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with null values");

        assert_eq!(response.date, "2026-01-28");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_views, 150);
        assert_eq!(summary.postviews.len(), 3);

        // First entry: all nullable fields are null
        let all_nulls = &summary.postviews[0];
        assert_eq!(all_nulls.id, 0);
        assert!(all_nulls.href.is_none());
        assert!(all_nulls.date.is_none());
        assert!(all_nulls.title.is_none());
        assert!(all_nulls.post_type.is_none());
        assert!(all_nulls.status.is_none());
        assert!(all_nulls.public.is_none());
        assert!(all_nulls.views.is_none());
        assert!(all_nulls.video_play.is_none());

        // Second entry: all fields have values
        let all_values = &summary.postviews[1];
        assert_eq!(all_values.id, 123);
        assert_eq!(
            all_values.href,
            Some("https://example.com/post".to_string())
        );
        assert_eq!(all_values.date, Some("2026-01-28 10:00:00".to_string()));
        assert_eq!(all_values.title, Some("A Post With All Fields".to_string()));
        assert_eq!(all_values.post_type, Some("post".to_string()));
        assert_eq!(all_values.status, Some("publish".to_string()));
        assert_eq!(all_values.public, Some(true));
        assert_eq!(all_values.views, Some(100));
        assert_eq!(all_values.video_play, Some(false));

        // Third entry: mixed null and non-null values
        let partial_nulls = &summary.postviews[2];
        assert_eq!(partial_nulls.id, 456);
        assert!(partial_nulls.href.is_none());
        assert!(partial_nulls.date.is_none());
        assert_eq!(partial_nulls.title, Some("Partial Null Post".to_string()));
        assert_eq!(partial_nulls.post_type, Some("post".to_string()));
        assert!(partial_nulls.status.is_none());
        assert_eq!(partial_nulls.public, Some(false));
        assert_eq!(partial_nulls.views, Some(50));
        assert!(partial_nulls.video_play.is_none());
    }

    #[test]
    fn test_stats_top_posts_empty_response_missing_dropped_ids() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-04-empty-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with missing dropped_ids");

        assert_eq!(response.date, "2026-01-28");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_views, 0);
        assert!(summary.postviews.is_empty());
        assert!(
            summary.dropped_ids.is_empty(),
            "dropped_ids should default to empty vec"
        );
    }
}
