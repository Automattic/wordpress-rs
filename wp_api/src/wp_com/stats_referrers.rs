use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Serialize};
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
}

impl AppendUrlQueryPairs for StatsReferrersParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("date", self.date.as_ref())
            .append_option_query_value_pair("start_date", self.start_date.as_ref())
            .append_option_query_value_pair("max", self.max.as_ref());
    }
}

/// Response from the stats referrers endpoint.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersResponse {
    /// The date for the stats query.
    pub date: String,
    /// The stats data grouped by day.
    pub days: HashMap<String, StatsReferrersDayData>,
    /// The time period used for grouping.
    pub period: String,
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

/// Returns all referrer groups from all days in the response.
#[uniffi::export]
pub fn get_stats_referrers_all_groups(
    response: &StatsReferrersResponse,
) -> Vec<StatsReferrersGroup> {
    response
        .days
        .values()
        .flat_map(|day_data| day_data.groups.clone())
        .collect()
}

/// Returns referrer data for a specific date.
#[uniffi::export]
pub fn get_stats_referrers_for_date(
    response: &StatsReferrersResponse,
    date: &str,
) -> Option<StatsReferrersDayData> {
    response.days.get(date).cloned()
}

/// Returns the total views across all days.
#[uniffi::export]
pub fn get_stats_referrers_total_views(response: &StatsReferrersResponse) -> u64 {
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
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers?period=day&date=2026-01-26&start_date=2026-01-26&max=10"
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
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/referrers?period=week&date=2026-01-19"
        );
    }

    #[test]
    fn test_stats_referrers_response_deserialization() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-26");
        assert_eq!(response.period, "day");
        assert!(response.days.contains_key("2026-01-26"));

        let day_data = response.days.get("2026-01-26").unwrap();
        assert_eq!(day_data.total_views, 22);
        assert_eq!(day_data.other_views, 0);
        assert_eq!(day_data.groups.len(), 6);

        // Verify first group (WordPress.com Reader)
        let first_group = &day_data.groups[0];
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

        let day_data = response.days.get("2026-01-26").unwrap();

        // Find Search Engines group (has detailed results array)
        let search_engines = day_data
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

        let day_data = response.days.get("2026-01-26").unwrap();

        // Find group with follow_data (dotcom.wordpress.com)
        let dotcom_group = day_data
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

        let all_groups = get_stats_referrers_all_groups(&response);

        assert_eq!(all_groups.len(), 6);
    }

    #[test]
    fn test_get_stats_referrers_for_date() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let day_data = get_stats_referrers_for_date(&response, "2026-01-26");
        assert!(day_data.is_some());
        let day_data = day_data.unwrap();
        assert_eq!(day_data.total_views, 22);

        let missing_data = get_stats_referrers_for_date(&response, "2026-01-27");
        assert!(missing_data.is_none());
    }

    #[test]
    fn test_get_stats_referrers_total_views() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let total = get_stats_referrers_total_views(&response);
        assert_eq!(total, 22);
    }

    #[test]
    fn test_stats_referrers_empty_response() {
        let response = StatsReferrersResponse {
            date: "2026-01-26".to_string(),
            days: HashMap::new(),
            period: "day".to_string(),
        };

        let all_groups = get_stats_referrers_all_groups(&response);
        assert!(all_groups.is_empty());

        let total = get_stats_referrers_total_views(&response);
        assert_eq!(total, 0);
    }
}
