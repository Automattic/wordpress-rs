use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// The time period for grouping referrers.
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
pub enum StatsReferrersPeriod {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl_as_query_value_from_to_string!(StatsReferrersPeriod);

/// Parameters for the stats referrers endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsReferrersParams {
    /// The time period for grouping stats.
    #[uniffi(default = None)]
    pub period: Option<StatsReferrersPeriod>,
    /// The date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub date: Option<String>,
    /// The start date to query stats for (format: YYYY-MM-DD).
    #[uniffi(default = None)]
    pub start_date: Option<String>,
    /// The maximum number of referrers to return.
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

impl AppendUrlQueryPairs for StatsReferrersParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
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

/// Response from the stats referrers endpoint.
///
/// The response structure varies based on the `summarize` parameter:
/// - When `summarize=1`: Contains `summary` field with aggregated data
/// - When `summarize` is not set: Contains `days` field with per-day data
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time period used for grouping (present when summarize=1).
    pub period: Option<String>,
    /// Summary data with aggregated referrer groups (present when summarize=1).
    pub summary: Option<StatsReferrersSummaryData>,
    /// Per-day stats data keyed by date string (present when summarize is not set).
    pub days: Option<HashMap<String, StatsReferrersDayData>>,
}

/// Summary data with aggregated referrer groups.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersSummaryData {
    /// The list of referrer groups.
    pub groups: Vec<StatsReferrersGroup>,
    /// Views from other referrers not included in the list.
    pub other_views: u64,
    /// The total number of views.
    pub total_views: u64,
}

/// Stats data for a single day.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersDayData {
    /// The list of referrer groups for this day.
    pub groups: Vec<StatsReferrersGroup>,
    /// Views from other referrers not included in the list.
    pub other_views: u64,
    /// The total number of views for this day.
    pub total_views: u64,
}

/// A referrer group entry in the stats referrers response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersGroup {
    /// The group identifier.
    pub group: String,
    /// The display name of the referrer.
    pub name: String,
    /// The URL of the referrer (optional for some groups like Search Engines).
    pub url: Option<String>,
    /// The icon URL for the referrer.
    pub icon: Option<String>,
    /// The total number of views from this referrer.
    pub total: u64,
    /// Follow data for WordPress.com sites (optional).
    /// The API can return `null`, `false`, or an object for this field.
    #[serde(default, deserialize_with = "deserialize_follow_data")]
    pub follow_data: Option<StatsReferrersFollowData>,
    /// The results data (can be simple views or detailed referrer list).
    pub results: StatsReferrersResults,
}

/// Results can be either a simple views object or a list of detailed referrers.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum StatsReferrersResults {
    /// Simple views count.
    Views(StatsReferrersViewsResult),
    /// Detailed list of referrers (e.g., for Search Engines group).
    Referrers(Vec<StatsReferrersDetailedResult>),
}

/// Simple views result.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersViewsResult {
    /// The number of views.
    pub views: u64,
}

/// Detailed referrer result (used in groups like Search Engines).
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersDetailedResult {
    /// The name of the specific referrer.
    pub name: String,
    /// The URL of the referrer.
    pub url: Option<String>,
    /// The icon URL for the referrer.
    pub icon: Option<String>,
    /// The number of views from this referrer.
    pub views: u64,
}

/// Follow data for WordPress.com sites.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersFollowData {
    /// The follow parameters.
    pub params: Option<StatsReferrersFollowParams>,
    /// The type of follow action.
    #[serde(rename = "type")]
    pub follow_type: Option<String>,
}

/// Deserializes follow_data which can be null, false, or an object.
/// The API sometimes returns `false` instead of `null` when follow data is not available.
fn deserialize_follow_data<'de, D>(
    deserializer: D,
) -> Result<Option<StatsReferrersFollowData>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    #[allow(clippy::large_enum_variant)]
    enum FollowDataOrBool {
        Data(StatsReferrersFollowData),
        #[allow(dead_code)]
        Bool(bool),
        Null,
    }

    match FollowDataOrBool::deserialize(deserializer)? {
        FollowDataOrBool::Data(data) => Ok(Some(data)),
        FollowDataOrBool::Bool(_) | FollowDataOrBool::Null => Ok(None),
    }
}

/// Parameters for following a referrer site.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersFollowParams {
    /// The stat source.
    #[serde(rename = "stat-source")]
    pub stat_source: Option<String>,
    /// Text for follow button.
    #[serde(rename = "follow-text")]
    pub follow_text: Option<String>,
    /// Text when following.
    #[serde(rename = "following-text")]
    pub following_text: Option<String>,
    /// Text on hover when following.
    #[serde(rename = "following-hover-text")]
    pub following_hover_text: Option<String>,
    /// The blog domain.
    pub blog_domain: Option<String>,
    /// The blog URL.
    pub blog_url: Option<String>,
    /// The blog ID.
    pub blog_id: Option<u64>,
    /// The site ID.
    pub site_id: Option<u64>,
    /// The blog title.
    pub blog_title: Option<String>,
    /// Whether currently following.
    pub is_following: Option<bool>,
}

/// Returns all referrer groups from the summary (when summarize=1 was used).
#[uniffi::export]
pub fn get_stats_referrers_all_groups(
    response: &StatsReferrersResponse,
) -> Option<Vec<StatsReferrersGroup>> {
    response.summary.as_ref().map(|s| s.groups.clone())
}

/// Returns the total views from the summary (when summarize=1 was used).
#[uniffi::export]
pub fn get_stats_referrers_total_views(response: &StatsReferrersResponse) -> Option<u64> {
    response.summary.as_ref().map(|s| s.total_views)
}

/// Returns the summary data (when summarize=1 was used).
#[uniffi::export]
pub fn get_stats_referrers_summary(
    response: &StatsReferrersResponse,
) -> Option<StatsReferrersSummaryData> {
    response.summary.clone()
}

/// Returns the days data (when summarize is not set).
#[uniffi::export]
pub fn get_stats_referrers_days(
    response: &StatsReferrersResponse,
) -> Option<HashMap<String, StatsReferrersDayData>> {
    response.days.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_referrers_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers",
        )
        .expect("Failed to parse url");

        let params = StatsReferrersParams {
            period: Some(StatsReferrersPeriod::Day),
            date: Some("2026-01-26".to_string()),
            start_date: Some("2026-01-26".to_string()),
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
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers?period=day&date=2026-01-26&start_date=2026-01-26&max=10&num=30&locale=en&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_referrers_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers",
        )
        .expect("Failed to parse url");

        let params = StatsReferrersParams {
            period: Some(StatsReferrersPeriod::Week),
            date: Some("2026-01-19".to_string()),
            start_date: None,
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
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers?period=week&date=2026-01-19&summarize=1&skip_archives=1"
        );
    }

    #[test]
    fn test_stats_referrers_params_without_summarize_and_skip_archives() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers",
        )
        .expect("Failed to parse url");

        let params = StatsReferrersParams {
            period: Some(StatsReferrersPeriod::Day),
            date: Some("2026-01-26".to_string()),
            summarize: None,
            skip_archives: None,
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers?period=day&date=2026-01-26"
        );
    }

    #[test]
    fn test_stats_referrers_params_with_false_values() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers",
        )
        .expect("Failed to parse url");

        let params = StatsReferrersParams {
            period: Some(StatsReferrersPeriod::Day),
            summarize: Some(false),
            skip_archives: Some(false),
            ..Default::default()
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers?period=day&summarize=0&skip_archives=0"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_referrers/referrers-01.json")]
    #[case("tests/wpcom/stats_referrers/referrers-03-follow-data-false.json")]
    #[case("tests/wpcom/stats_referrers/referrers-04-real-response.json")]
    fn test_stats_referrers_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(response.period.is_some());
        assert!(!response.period.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_stats_referrers_response_deserialization_referrers_01() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-26");
        assert_eq!(response.period, Some("day".to_string()));

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.total_views, 22);
        assert_eq!(summary.other_views, 0);
        assert_eq!(summary.groups.len(), 6);

        // Verify first group (WordPress.com Reader)
        let first_group = &summary.groups[0];
        assert_eq!(first_group.group, "WordPress.com Reader");
        assert_eq!(first_group.name, "WordPress.com Reader");
        assert_eq!(
            first_group.url,
            Some("https://wordpress.com/reader/".to_string())
        );
        assert_eq!(first_group.total, 12);
        assert!(first_group.follow_data.is_none());

        // Check simple views result
        match &first_group.results {
            StatsReferrersResults::Views(views) => {
                assert_eq!(views.views, 12);
            }
            _ => panic!("Expected Views result"),
        }
    }

    #[test]
    fn test_stats_referrers_search_engines_group() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        // Find Search Engines group (has detailed results array)
        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        let search_engines = summary
            .groups
            .iter()
            .find(|g| g.group == "Search Engines")
            .expect("Search Engines group should exist");

        assert_eq!(search_engines.name, "Search Engines");
        assert!(search_engines.url.is_none());
        assert_eq!(search_engines.total, 1);

        // Check detailed results
        match &search_engines.results {
            StatsReferrersResults::Referrers(referrers) => {
                assert_eq!(referrers.len(), 1);
                assert_eq!(referrers[0].name, "Google Search");
                assert_eq!(referrers[0].url, Some("http://www.google.com/".to_string()));
                assert_eq!(referrers[0].views, 1);
            }
            _ => panic!("Expected Referrers result"),
        }
    }

    #[test]
    fn test_stats_referrers_with_follow_data() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        // Find group with follow_data (dotcom.wordpress.com)
        let dotcom_group = summary
            .groups
            .iter()
            .find(|g| g.group == "dotcom.wordpress.com")
            .expect("dotcom.wordpress.com group should exist");

        assert!(dotcom_group.follow_data.is_some());
        let follow_data = dotcom_group.follow_data.as_ref().unwrap();
        assert_eq!(follow_data.follow_type, Some("follow".to_string()));

        let params = follow_data.params.as_ref().unwrap();
        assert_eq!(params.blog_domain, Some("dotcom.wordpress.com".to_string()));
        assert_eq!(params.blog_id, Some(19734));
        assert_eq!(params.site_id, Some(19734));
        assert_eq!(params.blog_title, Some("Dotcom P2".to_string()));
        assert_eq!(params.is_following, Some(true));
    }

    #[test]
    fn test_get_stats_referrers_all_groups() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let all_groups =
            get_stats_referrers_all_groups(&response).expect("Summary should be present");

        assert_eq!(all_groups.len(), 6);
    }

    #[test]
    fn test_get_stats_referrers_total_views() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let total = get_stats_referrers_total_views(&response).expect("Summary should be present");
        assert_eq!(total, 22);
    }

    #[test]
    fn test_get_stats_referrers_summary() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let summary = get_stats_referrers_summary(&response).expect("Summary should be present");
        assert_eq!(summary.total_views, 22);
        assert_eq!(summary.other_views, 0);
        assert_eq!(summary.groups.len(), 6);
    }

    #[rstest]
    #[case("tests/wpcom/stats_referrers/referrers-02-days.json")]
    fn test_stats_referrers_response_deserialization_days(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(response.summary.is_none());
        assert!(response.days.is_some());
    }

    #[test]
    fn test_stats_referrers_response_deserialization_days_02() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-02-days.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-26");
        assert!(response.period.is_none());
        assert!(response.summary.is_none());

        let days = response.days.as_ref().expect("Days should be present");
        assert_eq!(days.len(), 2);

        // Verify first day
        let day1 = days.get("2026-01-26").expect("2026-01-26 should exist");
        assert_eq!(day1.total_views, 13);
        assert_eq!(day1.other_views, 2);
        assert_eq!(day1.groups.len(), 2);

        // Verify first group of first day
        let first_group = &day1.groups[0];
        assert_eq!(first_group.group, "WordPress.com Reader");
        assert_eq!(first_group.total, 8);

        // Verify second day
        let day2 = days.get("2026-01-25").expect("2026-01-25 should exist");
        assert_eq!(day2.total_views, 5);
        assert_eq!(day2.other_views, 0);
        assert_eq!(day2.groups.len(), 1);
    }

    #[test]
    fn test_get_stats_referrers_days() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-02-days.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let days = get_stats_referrers_days(&response).expect("Days should be present");
        assert_eq!(days.len(), 2);
        assert!(days.contains_key("2026-01-26"));
        assert!(days.contains_key("2026-01-25"));
    }

    #[test]
    fn test_stats_referrers_summary_is_none_for_days_response() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-02-days.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(get_stats_referrers_summary(&response).is_none());
        assert!(get_stats_referrers_all_groups(&response).is_none());
        assert!(get_stats_referrers_total_views(&response).is_none());
    }

    #[test]
    fn test_stats_referrers_follow_data_as_false() {
        // The API can return `false` instead of `null` for follow_data
        let json_file_path = "tests/wpcom/stats_referrers/referrers-03-follow-data-false.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON with follow_data: false");

        let summary = response
            .summary
            .as_ref()
            .expect("Summary should be present");
        assert_eq!(summary.groups.len(), 2);

        // First group has follow_data: null
        let first_group = &summary.groups[0];
        assert_eq!(first_group.group, "WordPress.com Reader");
        assert!(
            first_group.follow_data.is_none(),
            "follow_data: null should be deserialized as None"
        );

        // Second group has follow_data: false
        let second_group = &summary.groups[1];
        assert_eq!(second_group.group, "domainsuniversity.wordpress.com");
        assert!(
            second_group.follow_data.is_none(),
            "follow_data: false should be deserialized as None"
        );
    }
}
