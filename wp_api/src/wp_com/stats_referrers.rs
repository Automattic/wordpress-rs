use crate::{
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Serialize};

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
    #[uniffi(default = None)]
    pub summarize: Option<bool>,
    /// Whether to skip archives in the response.
    #[uniffi(default = None)]
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
            .append_option_query_value_pair(
                "summarize",
                self.summarize.map(|b| b as u32).as_ref(),
            )
            .append_option_query_value_pair(
                "skip_archives",
                self.skip_archives.map(|b| b as u32).as_ref(),
            );
    }
}

/// Response from the stats referrers endpoint.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsReferrersResponse {
    /// The date for the stats query.
    pub date: String,
    /// The time period used for grouping.
    pub period: String,
    /// Summary data with aggregated referrer groups.
    pub summary: StatsReferrersSummaryData,
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

/// Returns all referrer groups from the summary.
#[uniffi::export]
pub fn get_stats_referrers_all_groups(
    response: &StatsReferrersResponse,
) -> Vec<StatsReferrersGroup> {
    response.summary.groups.clone()
}

/// Returns the total views from the summary.
#[uniffi::export]
pub fn get_stats_referrers_total_views(response: &StatsReferrersResponse) -> u64 {
    response.summary.total_views
}

/// Returns the summary data.
#[uniffi::export]
pub fn get_stats_referrers_summary(response: &StatsReferrersResponse) -> StatsReferrersSummaryData {
    response.summary.clone()
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
    fn test_stats_referrers_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(!response.date.is_empty());
        assert!(!response.period.is_empty());
    }

    #[test]
    fn test_stats_referrers_response_deserialization_referrers_01() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.date, "2026-01-26");
        assert_eq!(response.period, "day");
        assert_eq!(response.summary.total_views, 22);
        assert_eq!(response.summary.other_views, 0);
        assert_eq!(response.summary.groups.len(), 6);

        // Verify first group (WordPress.com Reader)
        let first_group = &response.summary.groups[0];
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
        let search_engines = response
            .summary
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

        // Find group with follow_data (dotcom.wordpress.com)
        let dotcom_group = response
            .summary
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
    fn test_get_stats_referrers_total_views() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let total = get_stats_referrers_total_views(&response);
        assert_eq!(total, 22);
    }

    #[test]
    fn test_get_stats_referrers_summary() {
        let json_file_path = "tests/wpcom/stats_referrers/referrers-01.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsReferrersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let summary = get_stats_referrers_summary(&response);
        assert_eq!(summary.total_views, 22);
        assert_eq!(summary.other_views, 0);
        assert_eq!(summary.groups.len(), 6);
    }
}
