use serde::{
    Deserialize, Serialize,
    de::{self, DeserializeOwned},
};
use std::collections::HashMap;

/// Deserialize a `HashMap` that may be represented as a non-map placeholder value.
///
/// The WP.com stats insights API returns `0`, `null`, or `false` instead of `{}`
/// when a site has no posting data. This handles all such cases by treating any
/// non-object JSON value as an empty `HashMap`.
fn deserialize_hashmap_or_placeholder_as_empty<'de, D, K, V>(
    deserializer: D,
) -> Result<HashMap<K, V>, D::Error>
where
    D: de::Deserializer<'de>,
    K: DeserializeOwned + std::hash::Hash + Eq,
    V: DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    if let serde_json::Value::Object(map) = value {
        serde_json::from_value(serde_json::Value::Object(map)).map_err(de::Error::custom)
    } else {
        Ok(HashMap::new())
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
    #[serde(deserialize_with = "deserialize_hashmap_or_placeholder_as_empty")]
    pub days: HashMap<String, u64>,
    /// Post counts by hour of day, keyed by hour string ("00" through "23").
    #[serde(deserialize_with = "deserialize_hashmap_or_placeholder_as_empty")]
    pub hours: HashMap<String, u64>,
    /// View counts by datetime, keyed by "YYYY-MM-DD HH:00:00" strings.
    #[serde(deserialize_with = "deserialize_hashmap_or_placeholder_as_empty")]
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
    use rstest::*;

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

    /// The WP.com API may return non-map values (integer, null, false) for map
    /// fields when a site has no data. All should deserialize as empty HashMaps.
    #[rstest]
    #[case("tests/wpcom/stats_insights/response-03-integer-fields.json")]
    #[case("tests/wpcom/stats_insights/response-04-null-fields.json")]
    #[case("tests/wpcom/stats_insights/response-05-false-fields.json")]
    fn test_stats_insights_response_deserialization_non_map_fields(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsInsightsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with non-map fields");

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
