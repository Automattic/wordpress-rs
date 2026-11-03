use crate::{
    WpApiParamOrder,
    date::WpDateString,
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
};
use serde::{Deserialize, Serialize};

/// The time period for emails summary stats.
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
pub enum StatsEmailsSummaryPeriod {
    Day,
    Week,
    Month,
    Year,
    #[default]
    #[serde(rename = "alltime")]
    #[strum(serialize = "alltime")]
    AllTime,
}

impl_as_query_value_from_to_string!(StatsEmailsSummaryPeriod);

/// The field to sort emails summary results by.
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
pub enum StatsEmailsSummarySortField {
    #[default]
    PostDate,
    Opens,
    Clicks,
}

impl_as_query_value_from_to_string!(StatsEmailsSummarySortField);

/// Parameters for the stats emails summary endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct StatsEmailsSummaryParams {
    /// The time period for the summary.
    #[uniffi(default = None)]
    pub period: Option<StatsEmailsSummaryPeriod>,
    /// The number of results to return.
    #[uniffi(default = None)]
    pub quantity: Option<u32>,
    /// The field to sort results by.
    #[uniffi(default = None)]
    pub sort_field: Option<StatsEmailsSummarySortField>,
    /// The sort order for results.
    #[uniffi(default = None)]
    pub sort_order: Option<WpApiParamOrder>,
}

impl AppendUrlQueryPairs for StatsEmailsSummaryParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("period", self.period.as_ref())
            .append_option_query_value_pair("quantity", self.quantity.as_ref())
            .append_option_query_value_pair("sort_field", self.sort_field.as_ref())
            .append_option_query_value_pair("sort_order", self.sort_order.as_ref());
    }
}

/// Response from the stats emails summary endpoint.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct StatsEmailsSummaryResponse {
    /// The list of email posts with their stats.
    #[serde(default)]
    pub posts: Vec<StatsEmailsSummaryPost>,
}

/// An email post entry in the stats emails summary response.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct StatsEmailsSummaryPost {
    /// The post ID.
    pub id: u64,
    /// The URL of the post.
    pub href: Option<String>,
    /// The publication date of the post, in the site's timezone.
    pub date: Option<WpDateString>,
    /// The title of the post.
    pub title: Option<String>,
    /// The type of the content (post, page, etc.).
    #[serde(rename = "type")]
    pub post_type: Option<String>,
    /// The number of email opens.
    pub opens: Option<u64>,
    /// The number of email clicks.
    pub clicks: Option<u64>,
    /// The open rate as a percentage.
    pub opens_rate: Option<f64>,
    /// The click rate as a percentage.
    pub clicks_rate: Option<f64>,
    /// The number of unique opens.
    pub unique_opens: Option<u64>,
    /// The number of unique clicks.
    pub unique_clicks: Option<u64>,
    /// The total number of sends.
    pub total_sends: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_stats_emails_summary_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/emails/summary",
        )
        .expect("Failed to parse url");

        let params = StatsEmailsSummaryParams {
            period: Some(StatsEmailsSummaryPeriod::AllTime),
            quantity: Some(30),
            sort_field: Some(StatsEmailsSummarySortField::PostDate),
            sort_order: Some(WpApiParamOrder::Desc),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/emails/summary?period=alltime&quantity=30&sort_field=post_date&sort_order=desc"
        );
    }

    #[test]
    fn test_stats_emails_summary_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/emails/summary",
        )
        .expect("Failed to parse url");

        let params = StatsEmailsSummaryParams {
            period: Some(StatsEmailsSummaryPeriod::Month),
            quantity: Some(10),
            sort_field: None,
            sort_order: None,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/emails/summary?period=month&quantity=10"
        );
    }

    #[test]
    fn test_stats_emails_summary_params_default() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/emails/summary",
        )
        .expect("Failed to parse url");

        let params = StatsEmailsSummaryParams::default();

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/rest/v1.1/sites/1234/stats/emails/summary?"
        );
    }

    #[rstest]
    #[case("tests/wpcom/stats_emails_summary/response-01-with-posts.json")]
    #[case("tests/wpcom/stats_emails_summary/response-02-empty.json")]
    #[case("tests/wpcom/stats_emails_summary/response-03-with-nulls.json")]
    fn test_stats_emails_summary_response_deserialization(#[case] json_file_path: &str) {
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let _response: StatsEmailsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
    }

    #[test]
    fn test_stats_emails_summary_response_with_posts() {
        let json_file_path = "tests/wpcom/stats_emails_summary/response-01-with-posts.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsEmailsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.posts.len(), 3);

        let first_post = &response.posts[0];
        assert_eq!(first_post.id, 210454);
        assert_eq!(
            first_post.href,
            Some("https://example.com/post-1".to_string())
        );
        assert_eq!(
            first_post.date,
            Some(WpDateString::new("2023-08-17 15:40:59".to_string()))
        );
        assert_eq!(first_post.title, Some("Example Post Title".to_string()));
        assert_eq!(first_post.post_type, Some("post".to_string()));
        assert_eq!(first_post.opens, Some(13));
        assert_eq!(first_post.clicks, Some(1));
        assert_eq!(first_post.opens_rate, Some(0.0));
        assert_eq!(first_post.clicks_rate, Some(0.0));
        assert_eq!(first_post.unique_opens, Some(0));
        assert_eq!(first_post.unique_clicks, Some(0));
        assert_eq!(first_post.total_sends, Some(4));
    }

    #[test]
    fn test_stats_emails_summary_response_with_rates() {
        let json_file_path = "tests/wpcom/stats_emails_summary/response-01-with-posts.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsEmailsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        let post_with_rates = &response.posts[2];
        assert_eq!(post_with_rates.id, 210796);
        assert_eq!(post_with_rates.opens_rate, Some(33.33));
        assert_eq!(post_with_rates.clicks_rate, Some(0.0));
        assert_eq!(post_with_rates.unique_opens, Some(1));
    }

    #[test]
    fn test_stats_emails_summary_empty_response() {
        let json_file_path = "tests/wpcom/stats_emails_summary/response-02-empty.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsEmailsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert!(response.posts.is_empty());
    }

    #[test]
    fn test_stats_emails_summary_with_null_values() {
        let json_file_path = "tests/wpcom/stats_emails_summary/response-03-with-nulls.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: StatsEmailsSummaryResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.posts.len(), 2);

        // First entry: all nullable fields are null
        let all_nulls = &response.posts[0];
        assert_eq!(all_nulls.id, 100);
        assert!(all_nulls.href.is_none());
        assert!(all_nulls.date.is_none());
        assert!(all_nulls.title.is_none());
        assert!(all_nulls.post_type.is_none());
        assert!(all_nulls.opens.is_none());
        assert!(all_nulls.clicks.is_none());
        assert!(all_nulls.opens_rate.is_none());
        assert!(all_nulls.clicks_rate.is_none());
        assert!(all_nulls.unique_opens.is_none());
        assert!(all_nulls.unique_clicks.is_none());
        assert!(all_nulls.total_sends.is_none());

        // Second entry: all fields present
        let all_values = &response.posts[1];
        assert_eq!(all_values.id, 200);
        assert_eq!(
            all_values.href,
            Some("https://example.com/post".to_string())
        );
        assert_eq!(all_values.title, Some("A Post".to_string()));
        assert_eq!(all_values.opens, Some(5));
        assert_eq!(all_values.clicks, Some(2));
    }
}
