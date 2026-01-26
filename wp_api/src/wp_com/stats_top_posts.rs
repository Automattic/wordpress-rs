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
}

impl AppendUrlQueryPairs for StatsTopPostsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref());
    }
}

/// Response from the stats top posts endpoint.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopPostsResponse {
    /// The date for the stats query.
    pub date: String,
    /// The stats data grouped by day.
    pub days: HashMap<String, StatsTopPostsDayData>,
    /// The time period used for grouping.
    pub period: String,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopPostsDayData {
    /// The list of post views for this day.
    pub postviews: Vec<StatsTopPostsPostView>,
    /// The total number of views for this day.
    pub total_views: u64,
    /// IDs that were dropped from the results.
    pub dropped_ids: Vec<u64>,
    /// Views from other posts not included in the list.
    pub other_views: u64,
}

/// A post view entry in the stats top posts response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopPostsPostView {
    /// The post ID.
    pub id: u64,
    /// The URL of the post.
    pub href: String,
    /// The publication date of the post (can be null for homepage).
    pub date: Option<String>,
    /// The title of the post.
    pub title: String,
    /// The type of the content (post, page, homepage, etc.).
    #[serde(rename = "type")]
    pub post_type: String,
    /// The publication status (can be null for homepage).
    pub status: Option<String>,
    /// Whether the post is public.
    pub public: bool,
    /// The number of views.
    pub views: u64,
    /// Whether this is a video play.
    pub video_play: bool,
}

/// Returns all post views from all days in the response.
#[uniffi::export]
pub fn get_stats_top_posts_all_post_views(
    response: &StatsTopPostsResponse,
) -> Vec<StatsTopPostsPostView> {
    response
        .days
        .values()
        .flat_map(|day_data| day_data.postviews.clone())
        .collect()
}

/// Returns post views for a specific date.
#[uniffi::export]
pub fn get_stats_top_posts_for_date(
    response: &StatsTopPostsResponse,
    date: &str,
) -> Option<StatsTopPostsDayData> {
    response.days.get(date).cloned()
}

/// Returns the total views across all days.
#[uniffi::export]
pub fn get_stats_top_posts_total_views(response: &StatsTopPostsResponse) -> u64 {
    response
        .days
        .values()
        .map(|day_data| day_data.total_views)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts?period=day&start_date=2026-01-26&date=2026-01-26&max=10"
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
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-posts?period=week&date=2026-01-19"
        );
    }

    #[test]
    fn test_stats_top_posts_response_deserialization() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-25");
        assert_eq!(response.period, "week");
        assert!(response.days.contains_key("2026-01-19"));

        let day_data = response.days.get("2026-01-19").unwrap();
        assert_eq!(day_data.total_views, 2996);
        assert_eq!(day_data.other_views, 2040);
        assert!(day_data.dropped_ids.is_empty());
        assert_eq!(day_data.postviews.len(), 10);

        // Verify first post view
        let first_post = &day_data.postviews[0];
        assert_eq!(first_post.id, 269);
        assert_eq!(first_post.title, "Welcome to Automattic");
        assert_eq!(first_post.post_type, "page");
        assert_eq!(first_post.status, Some("publish".to_string()));
        assert!(first_post.public);
        assert_eq!(first_post.views, 417);
        assert!(!first_post.video_play);
    }

    #[test]
    fn test_stats_top_posts_homepage_entry() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let day_data = response.days.get("2026-01-19").unwrap();

        // Find homepage entry (id: 0, type: homepage, date: null, status: null)
        let homepage = day_data
            .postviews
            .iter()
            .find(|p| p.post_type == "homepage")
            .expect("Homepage entry should exist");

        assert_eq!(homepage.id, 0);
        assert_eq!(homepage.title, "Home page / Archives");
        assert!(homepage.date.is_none());
        assert!(homepage.status.is_none());
        assert!(!homepage.public);
        assert_eq!(homepage.views, 244);
    }

    #[test]
    fn test_get_stats_top_posts_all_post_views() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let all_posts = get_stats_top_posts_all_post_views(&response);

        assert_eq!(all_posts.len(), 10);
    }

    #[test]
    fn test_get_stats_top_posts_for_date() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let day_data = get_stats_top_posts_for_date(&response, "2026-01-19");
        assert!(day_data.is_some());
        let day_data = day_data.unwrap();
        assert_eq!(day_data.total_views, 2996);

        let missing_data = get_stats_top_posts_for_date(&response, "2026-01-20");
        assert!(missing_data.is_none());
    }

    #[test]
    fn test_get_stats_top_posts_total_views() {
        let json_file_path = "tests/wpcom/stats_top_posts/top-posts-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopPostsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let total = get_stats_top_posts_total_views(&response);
        assert_eq!(total, 2996);
    }

    #[test]
    fn test_stats_top_posts_empty_response() {
        let response = StatsTopPostsResponse {
            date: "2026-01-26".to_string(),
            days: HashMap::new(),
            period: "day".to_string(),
        };

        let all_posts = get_stats_top_posts_all_post_views(&response);
        assert!(all_posts.is_empty());

        let total = get_stats_top_posts_total_views(&response);
        assert_eq!(total, 0);
    }
}
