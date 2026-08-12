use crate::{
    date::{WpDateString, WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    wp_com::{language::WPComLanguage, stats_visits::StatsVisitsResponse},
};
use serde::{Deserialize, Serialize};

/// Parameters for the stats summary endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsSummaryParams {
    /// The locale for the response.
    #[uniffi(default = None)]
    pub locale: Option<WPComLanguage>,
}

impl AppendUrlQueryPairs for StatsSummaryParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair("locale", self.locale.as_ref());
    }
}

/// Response from the stats summary endpoint.
///
/// Contains the site's aggregate statistics and recent visit time-series data.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSummaryResponse {
    /// The date of the stats query.
    pub date: WpDateString,
    /// Aggregate site statistics.
    pub stats: StatsSummaryStats,
    /// Recent visit time-series data.
    pub visits: StatsVisitsResponse,
}

/// Aggregate site statistics.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsSummaryStats {
    /// Number of visitors today.
    pub visitors_today: u64,
    /// Number of visitors yesterday.
    pub visitors_yesterday: u64,
    /// Total number of visitors.
    pub visitors: u64,
    /// Number of views today.
    pub views_today: u64,
    /// Number of views yesterday.
    pub views_yesterday: u64,
    /// The date with the most views.
    pub views_best_day: WpDateString,
    /// The total views on the best day.
    pub views_best_day_total: u64,
    /// Total number of views.
    pub views: u64,
    /// Total number of comments.
    pub comments: u64,
    /// Total number of posts.
    pub posts: u64,
    /// Number of blog followers.
    pub followers_blog: u64,
    /// Number of comment followers.
    pub followers_comments: u64,
    /// Average comments per month.
    pub comments_per_month: u64,
    /// The most active recent day for comments. The API derives this from a
    /// comment's GMT timestamp, so it carries a time of day that isn't part of
    /// what the field reports.
    ///
    /// An empty string, `null`, or an absent field all read as `None`.
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    pub comments_most_active_recent_day: Option<WpGmtDateTime>,
    /// The most active time for comments.
    /// `"N/A"` values from the API are treated as `None`.
    #[serde(deserialize_with = "wp_serde_helper::deserialize_placeholder_string_as_none")]
    pub comments_most_active_time: Option<String>,
    /// Number of spam comments.
    pub comments_spam: u64,
    /// Total number of categories.
    pub categories: u64,
    /// Total number of tags.
    pub tags: u64,
    /// Total number of shares.
    pub shares: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_summary_params_serialization_with_locale() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats")
                .expect("Failed to parse url");

        let params = StatsSummaryParams {
            locale: Some(WPComLanguage::Spanish),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats?locale=es"
        );
    }

    #[test]
    fn test_stats_summary_params_serialization_default() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats")
                .expect("Failed to parse url");

        let params = StatsSummaryParams::default();

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats?"
        );
    }

    #[test]
    fn test_stats_summary_response_deserialization() {
        let json_file_path = "tests/wpcom/stats_summary/response-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-03-10");

        // Verify stats
        assert_eq!(response.stats.visitors_today, 222);
        assert_eq!(response.stats.visitors_yesterday, 345);
        assert_eq!(response.stats.visitors, 154791);
        assert_eq!(response.stats.views_today, 745);
        assert_eq!(response.stats.views_yesterday, 1405);
        assert_eq!(response.stats.views_best_day.0, "2022-02-22");
        assert_eq!(response.stats.views_best_day_total, 4615);
        assert_eq!(response.stats.views, 6782783);
        assert_eq!(response.stats.comments, 0);
        assert_eq!(response.stats.posts, 2);
        assert_eq!(response.stats.followers_blog, 89);
        assert_eq!(response.stats.followers_comments, 4);
        assert_eq!(response.stats.comments_per_month, 0);
        assert_eq!(response.stats.comments_most_active_recent_day, None);
        assert_eq!(response.stats.comments_most_active_time, None);
        assert_eq!(response.stats.comments_spam, 0);
        assert_eq!(response.stats.categories, 473);
        assert_eq!(response.stats.tags, 1403);
        assert_eq!(response.stats.shares, 1);

        // Verify visits
        assert_eq!(response.visits.unit, "day");
        assert_eq!(response.visits.data.len(), 30);
    }

    #[test]
    fn test_stats_summary_visits_data() {
        let json_file_path = "tests/wpcom/stats_summary/response-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let visits = response.visits.visits_data();
        assert_eq!(visits.len(), 30);
        assert_eq!(visits[0].period.0, "2026-02-09");
        assert_eq!(visits[0].visits, 1384);

        let visitors = response.visits.visitors_data();
        assert_eq!(visitors.len(), 30);
        assert_eq!(visitors[0].period.0, "2026-02-09");
        assert_eq!(visitors[0].visitors, 376);
    }

    #[test]
    fn test_stats_summary_zero_stats_deserialization() {
        let json_file_path = "tests/wpcom/stats_summary/response-02-zero-stats.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date.0, "2026-03-10");

        // Verify all stats are zero
        assert_eq!(response.stats.visitors_today, 0);
        assert_eq!(response.stats.visitors_yesterday, 0);
        assert_eq!(response.stats.visitors, 0);
        assert_eq!(response.stats.views_today, 0);
        assert_eq!(response.stats.views_yesterday, 0);
        assert_eq!(response.stats.views_best_day.0, "");
        assert_eq!(response.stats.views_best_day_total, 0);
        assert_eq!(response.stats.views, 0);
        assert_eq!(response.stats.comments, 0);
        assert_eq!(response.stats.posts, 0);
        assert_eq!(response.stats.followers_blog, 0);
        assert_eq!(response.stats.followers_comments, 0);
        assert_eq!(response.stats.comments_per_month, 0);
        assert_eq!(response.stats.comments_most_active_recent_day, None);
        assert_eq!(response.stats.comments_most_active_time, None);
        assert_eq!(response.stats.comments_spam, 0);
        assert_eq!(response.stats.categories, 0);
        assert_eq!(response.stats.tags, 0);
        assert_eq!(response.stats.shares, 0);

        // Verify empty visits
        assert_eq!(response.visits.unit, "day");
        assert_eq!(response.visits.data.len(), 0);
        assert!(response.visits.visits_data().is_empty());
        assert!(response.visits.visitors_data().is_empty());
    }
}
