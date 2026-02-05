use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::language::WPComLanguage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The time period for grouping top authors stats.
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
pub enum StatsTopAuthorsPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsTopAuthorsPeriod);

/// Parameters for the stats top authors endpoint.
#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatsTopAuthorsParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsTopAuthorsPeriod>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The maximum number of top authors to return.
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
}

impl Default for StatsTopAuthorsParams {
    fn default() -> Self {
        Self {
            period: None,
            start_date: None,
            date: None,
            max: None,
            num: None,
            locale: None,
            summarize: true,
        }
    }
}

impl AppendUrlQueryPairs for StatsTopAuthorsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref())
            .append_option_query_value_pair("num", self.num.as_ref())
            .append_option_query_value_pair("locale", self.locale.as_ref())
            .append_query_value_pair("summarize", &(self.summarize as u32));
    }
}

/// Response from the stats top authors endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize` is not set: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopAuthorsResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time period used for grouping (present when summarize=1).
    pub period: Option<String>,
    /// Summary data with aggregated author views (present when summarize=1).
    pub summary: Option<StatsTopAuthorsSummaryData>,
    /// Per-day stats data keyed by date string (present when summarize is not set).
    pub days: Option<HashMap<String, StatsTopAuthorsDayData>>,
}

/// Summary data with aggregated author views.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopAuthorsSummaryData {
    /// The list of author stats.
    pub authors: Vec<StatsTopAuthorsAuthor>,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopAuthorsDayData {
    /// The list of author stats for this day.
    pub authors: Vec<StatsTopAuthorsAuthor>,
}

/// An author entry in the stats top authors response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopAuthorsAuthor {
    /// The author's display name.
    pub name: String,
    /// The author's avatar URL.
    pub avatar: Option<String>,
    /// The total number of views for this author.
    pub views: u64,
    /// The author's posts with their view counts.
    #[serde(default)]
    pub posts: Vec<StatsTopAuthorsPost>,
    /// Follow data for the author.
    pub follow_data: Option<StatsTopAuthorsFollowData>,
    /// The author's user ID.
    pub author_id: Option<u64>,
    /// Views from other posts not included in the posts list.
    pub other_views: Option<u64>,
}

/// A post entry in the stats top authors response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopAuthorsPost {
    /// The post ID.
    pub id: u64,
    /// The title of the post.
    pub title: Option<String>,
    /// The URL of the post.
    pub url: Option<String>,
    /// The number of views for this post.
    pub views: u64,
}

/// Follow data for an author.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopAuthorsFollowData {
    /// The type of follow.
    #[serde(rename = "type")]
    pub follow_type: Option<String>,
    /// Additional follow parameters.
    pub params: Option<StatsTopAuthorsFollowParams>,
}

/// Follow parameters for an author.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsTopAuthorsFollowParams {
    /// The feed ID.
    pub feed_id: Option<u64>,
    /// The blog ID.
    pub blog_id: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_top_authors_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-authors",
        )
        .expect("Failed to parse url");

        let params = StatsTopAuthorsParams {
            period: Some(StatsTopAuthorsPeriod::Day),
            start_date: Some("2026-01-30".to_string()),
            date: Some("2026-02-05".to_string()),
            max: Some(10),
            num: Some(7),
            locale: Some(WPComLanguage::English),
            summarize: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-authors?period=day&start_date=2026-01-30&date=2026-02-05&max=10&num=7&locale=en&summarize=1"
        );
    }

    #[test]
    fn test_stats_top_authors_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-authors",
        )
        .expect("Failed to parse url");

        let params = StatsTopAuthorsParams {
            period: Some(StatsTopAuthorsPeriod::Week),
            start_date: None,
            date: Some("2026-02-05".to_string()),
            max: None,
            num: None,
            locale: None,
            summarize: true,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-authors?period=week&date=2026-02-05&summarize=1"
        );
    }

    #[test]
    fn test_stats_top_authors_params_without_summarize() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-authors",
        )
        .expect("Failed to parse url");

        let params = StatsTopAuthorsParams {
            period: Some(StatsTopAuthorsPeriod::Day),
            date: Some("2026-02-05".to_string()),
            summarize: false,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/top-authors?period=day&date=2026-02-05&summarize=0"
        );
    }

    #[test]
    fn test_stats_top_authors_params_default() {
        let params = StatsTopAuthorsParams::default();
        assert!(params.summarize);
        assert!(params.period.is_none());
        assert!(params.date.is_none());
        assert!(params.start_date.is_none());
        assert!(params.max.is_none());
        assert!(params.num.is_none());
        assert!(params.locale.is_none());
    }

    #[rstest]
    #[case("tests/wpcom/stats_top_authors/summarized-01-day.json")]
    #[case("tests/wpcom/stats_top_authors/summarized-02-week.json")]
    fn test_stats_top_authors_response_deserialization_summary(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopAuthorsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(response.period.is_some());
        assert!(!response.period.as_ref().unwrap().is_empty());

        response
            .summary
            .as_ref()
            .expect("Summary should be present");
    }

    #[test]
    fn test_stats_top_authors_response_deserialization_summary_details() {
        let json_file_path = "tests/wpcom/stats_top_authors/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopAuthorsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-05");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert!(!summary.authors.is_empty());

        // Verify first author
        let first_author = &summary.authors[0];
        assert!(!first_author.name.is_empty());
        assert!(first_author.views > 0);
        assert!(!first_author.posts.is_empty());

        // Verify first post
        let first_post = &first_author.posts[0];
        assert!(first_post.id > 0);
        assert!(first_post.title.is_some());
        assert!(first_post.views > 0);
    }

    #[rstest]
    #[case("tests/wpcom/stats_top_authors/no-summary-01.json")]
    fn test_stats_top_authors_response_deserialization_days(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopAuthorsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(response.summary.is_none());
        assert!(response.days.is_some());
    }

    #[test]
    fn test_stats_top_authors_response_deserialization_days_details() {
        let json_file_path = "tests/wpcom/stats_top_authors/no-summary-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopAuthorsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-02-05");
        assert!(response.period.is_none());
        assert!(response.summary.is_none());

        let days = response.days.as_ref().expect("Days should be present");
        assert!(!days.is_empty());

        // Verify at least one day has authors
        let first_day = days.values().next().expect("Should have at least one day");
        assert!(!first_day.authors.is_empty());
    }

    #[test]
    fn test_stats_top_authors_author_with_follow_data() {
        let json_file_path = "tests/wpcom/stats_top_authors/summarized-01-day.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsTopAuthorsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");

        // Find an author with follow_data
        let author_with_follow = summary
            .authors
            .iter()
            .find(|a| a.follow_data.is_some())
            .expect("Should have author with follow_data");

        let follow_data = author_with_follow.follow_data.as_ref().unwrap();
        assert!(follow_data.follow_type.is_some());
    }
}
