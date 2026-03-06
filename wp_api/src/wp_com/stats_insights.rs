use crate::url_query::{AppendUrlQueryPairs, QueryPairs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::deserialize_empty_array_or_hashmap;

/// Parameters for the stats insights endpoint.
///
/// The insights endpoint does not accept any query parameters.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsInsightsParams {}

impl AppendUrlQueryPairs for StatsInsightsParams {
    fn append_query_pairs(&self, _query_pairs_mut: &mut QueryPairs) {
        // No query parameters for this endpoint
    }
}

/// Response from the stats insights endpoint.
///
/// Contains posting activity patterns including the best hour and day to post,
/// hourly view breakdowns, and yearly posting summaries.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsInsightsResponse {
    /// The hour of the day (0-23) with the highest posting activity.
    pub highest_hour: u32,
    /// The percentage of posts published during the highest hour.
    pub highest_hour_percent: f64,
    /// The day of the week (0=Sunday, 6=Saturday) with the highest posting activity.
    pub highest_day_of_week: u32,
    /// The percentage of posts published on the highest day.
    pub highest_day_percent: f64,
    /// Post counts by day of week, keyed by day index ("0"=Sunday through "6"=Saturday).
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub days: HashMap<String, u64>,
    /// Post counts by hour of day, keyed by hour string ("00" through "23").
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub hours: HashMap<String, u64>,
    /// View counts by datetime, keyed by "YYYY-MM-DD HH:00:00" strings.
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub hourly_views: HashMap<String, u64>,
    /// Yearly posting summaries.
    pub years: Vec<StatsInsightsYearData>,
}

/// Yearly posting summary data.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsInsightsYearData {
    /// The year as a string (e.g. "2024").
    pub year: String,
    /// Total number of posts published in this year.
    pub total_posts: u64,
    /// Total number of words written in this year.
    pub total_words: u64,
    /// Average number of words per post.
    pub avg_words: f64,
    /// Total number of likes received in this year.
    pub total_likes: u64,
    /// Average number of likes per post.
    pub avg_likes: f64,
    /// Total number of comments received in this year.
    pub total_comments: u64,
    /// Average number of comments per post.
    pub avg_comments: f64,
    /// Total number of images used in this year.
    pub total_images: u64,
    /// Average number of images per post.
    pub avg_images: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_insights_params_serialization() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/insights")
                .expect("Failed to parse url");

        let params = StatsInsightsParams {};

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/insights?"
        );
    }

    #[test]
    fn test_stats_insights_response_deserialization() {
        let json_file_path = "tests/wpcom/stats_insights/response-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.highest_hour, 16);
        assert!((response.highest_hour_percent - 9.588268471517203).abs() < 1e-10);
        assert_eq!(response.highest_day_of_week, 0);
        assert!((response.highest_day_percent - 24.946865037194474).abs() < 1e-10);
    }

    #[test]
    fn test_stats_insights_response_deserialization_empty_arrays() {
        let json_file_path = "tests/wpcom/stats_insights/response-02-empty.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with empty arrays");

        assert_eq!(response.highest_hour, 0);
        assert!(response.days.is_empty());
        assert!(response.hours.is_empty());
        assert!(response.hourly_views.is_empty());
        assert!(response.years.is_empty());
    }

    #[test]
    fn test_stats_insights_response_deserialization_integer_fields() {
        let json_file_path = "tests/wpcom/stats_insights/response-03-integer-fields.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with integer fields");

        assert_eq!(response.highest_hour, 0);
        assert!(response.days.is_empty());
        assert!(response.hours.is_empty());
        assert!(response.hourly_views.is_empty());
        assert!(response.years.is_empty());
    }

    #[test]
    fn test_stats_insights_days() {
        let json_file_path = "tests/wpcom/stats_insights/response-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.days.len(), 7);
        assert_eq!(response.days.get("0"), Some(&939));
        assert_eq!(response.days.get("6"), Some(&77));
    }

    #[test]
    fn test_stats_insights_hours() {
        let json_file_path = "tests/wpcom/stats_insights/response-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.hours.len(), 24);
        assert_eq!(response.hours.get("16"), Some(&340));
        assert_eq!(response.hours.get("05"), Some(&15));
    }

    #[test]
    fn test_stats_insights_hourly_views() {
        let json_file_path = "tests/wpcom/stats_insights/response-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.hourly_views.is_empty());
        assert_eq!(response.hourly_views.get("2026-03-04 16:00:00"), Some(&13));
        assert_eq!(response.hourly_views.get("2026-03-06 14:00:00"), Some(&10));
    }

    #[test]
    fn test_stats_insights_years() {
        let json_file_path = "tests/wpcom/stats_insights/response-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.years.len(), 3);

        let year_2024 = &response.years[0];
        assert_eq!(year_2024.year, "2024");
        assert_eq!(year_2024.total_posts, 54);
        assert_eq!(year_2024.total_words, 17252);
        assert!((year_2024.avg_words - 319.5).abs() < 1e-10);
        assert_eq!(year_2024.total_likes, 464);
        assert!((year_2024.avg_likes - 8.6).abs() < 1e-10);
        assert_eq!(year_2024.total_comments, 361);
        assert!((year_2024.avg_comments - 6.7).abs() < 1e-10);
        assert_eq!(year_2024.total_images, 26);
        assert!((year_2024.avg_images - 1.9).abs() < 1e-10);

        let year_2026 = &response.years[2];
        assert_eq!(year_2026.year, "2026");
        assert_eq!(year_2026.total_posts, 72);
    }
}
