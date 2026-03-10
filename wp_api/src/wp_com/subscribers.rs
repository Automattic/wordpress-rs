use crate::{
    WpApiParamOrder,
    date::WpGmtDateTime,
    impl_as_query_value_for_new_type, impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    users::UserId,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Not};
use wp_serde_helper::{deserialize_false_or_string, deserialize_u64_or_string};

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct Subscriber {
    pub user_id: UserId,
    pub subscription_id: SubscriptionId,
    pub display_name: String,
    pub email_address: String,
    pub is_email_subscriber: bool,
    pub date_subscribed: WpGmtDateTime,
    pub subscription_status: Option<String>,
    pub avatar: String,
    pub url: Option<String>,
    pub country: Option<SubscriberCountry>,
    pub plans: Option<Vec<SubscriptionPlan>>,
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum SubscriberType {
    All,
    WpCom,
    Email,
    Paid,
    Free,
    EmailSubscriber,
    ReaderSubscriber,
    UnconfirmedSubscriber,
    BlockedSubscriber,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(SubscriberType);

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
pub enum SubscriptionStatus {
    Active,
    Pending,
    Unsubscribed,
    Spam,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SubscriberCountry {
    #[serde(default, deserialize_with = "deserialize_false_or_string")]
    code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_false_or_string")]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SubscriptionPlan {
    pub is_gift: bool,
    pub gift_id: Option<u64>,
    pub paid_subscription_id: Option<String>,
    pub status: String,
    pub title: String,
    pub currency: String,
    pub renew_interval: String,
    pub inactive_renew_interval: Option<String>,
    pub renewal_price: f64,
    pub start_date: WpGmtDateTime,
    pub end_date: WpGmtDateTime,
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct SubscribersListParams {
    /// The current page.
    #[uniffi(default = None)]
    pub page: Option<u64>,
    /// The amount of items to show per page.
    #[uniffi(default = None)]
    pub per_page: Option<u64>,
    /// Search for subscribers
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Sort subscribers by a specific field
    #[uniffi(default = None)]
    pub sort: Option<ListSubscribersSortField>,
    /// Sort order
    #[uniffi(default = None)]
    pub sort_order: Option<WpApiParamOrder>,
    /// Filter subscribers by a specific subscriber type
    #[uniffi(default = None)]
    pub filter: Option<SubscriberType>,
    /// Array of filters to apply (combined with AND logic). If provided, overrides the single filter parameter.
    #[uniffi(default = None)]
    pub filters: Option<Vec<SubscriberType>>,
    /// An array of additional fields to include
    #[uniffi(default = [])]
    pub include: Vec<ListSubscribersIncludeField>,
}

impl AppendUrlQueryPairs for SubscribersListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("page", self.page.as_ref())
            .append_option_query_value_pair("per_page", self.per_page.as_ref())
            .append_option_query_value_pair("search", self.search.as_ref())
            .append_option_query_value_pair("sort", self.sort.as_ref())
            .append_option_query_value_pair("sort_order", self.sort_order.as_ref())
            .append_vec_query_value_pair("include", self.include.as_ref());

        if let Some(filters) = &self.filters {
            query_pairs_mut.append_pair(
                "filters",
                &filters
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            );
        } else if let Some(filter) = &self.filter {
            query_pairs_mut.append_pair("filter", &filter.to_string());
        }
    }
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum ListSubscribersSortField {
    DateSubscribed,
    #[strum(serialize = "email")]
    EmailAddress,
    #[strum(serialize = "name")]
    DisplayName,
    Plan,
    SubscriptionStatus,
}

impl_as_query_value_from_to_string!(ListSubscribersSortField);

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum ListSubscribersIncludeField {
    Country,
}

impl_as_query_value_from_to_string!(ListSubscribersIncludeField);

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ListSubscribersResponse {
    pub total: u64,
    pub pages: u64,
    pub page: u64,
    pub per_page: u64,
    pub subscribers: Vec<Subscriber>,
    pub is_owner_subscribed: bool,
}

// MARK: - List Subscribers by User Type

/// The user type to filter subscribers by.
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
pub enum SubscribersByUserTypeUserType {
    #[default]
    #[serde(rename = "wpcom")]
    #[strum(serialize = "wpcom")]
    WpCom,
    Email,
    Paid,
    Free,
}

impl_as_query_value_from_to_string!(SubscribersByUserTypeUserType);

/// The field to sort subscribers by user type results by.
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
pub enum SubscribersByUserTypeSortField {
    #[default]
    DateSubscribed,
    #[serde(rename = "email")]
    #[strum(serialize = "email")]
    EmailAddress,
    #[serde(rename = "name")]
    #[strum(serialize = "name")]
    DisplayName,
}

impl_as_query_value_from_to_string!(SubscribersByUserTypeSortField);

/// Parameters for the subscribers by user type endpoint.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct SubscribersByUserTypeParams {
    /// The number of subscribers per page.
    #[uniffi(default = None)]
    pub per_page: Option<u64>,
    /// The page number to return.
    #[uniffi(default = None)]
    pub page: Option<u64>,
    /// The user type to filter by.
    #[uniffi(default = None)]
    pub user_type: Option<SubscribersByUserTypeUserType>,
    /// The field to sort results by.
    #[uniffi(default = None)]
    pub sort: Option<SubscribersByUserTypeSortField>,
}

impl AppendUrlQueryPairs for SubscribersByUserTypeParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("per_page", self.per_page.as_ref())
            .append_option_query_value_pair("page", self.page.as_ref())
            .append_option_query_value_pair("user_type", self.user_type.as_ref())
            .append_option_query_value_pair("sort", self.sort.as_ref());
    }
}

// MARK: - Get Subscriber

#[derive(Debug, uniffi::Enum)]
pub enum IndividualSubscriberParams {
    // Return subscribers that receive notifications via WordPress.com for new posts.
    WpCom(UserId),

    // Return subscribers that receive notifications via email for new posts.
    Email(SubscriptionId),
}

impl AppendUrlQueryPairs for IndividualSubscriberParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        match self {
            IndividualSubscriberParams::WpCom(user_id) => {
                query_pairs_mut.append_pair("user_id", &user_id.to_string());
                query_pairs_mut.append_pair("type", "wpcom");
            }
            IndividualSubscriberParams::Email(email) => {
                query_pairs_mut.append_pair("subscription_id", &email.to_string());
                query_pairs_mut.append_pair("type", "email");
            }
        }
    }
}

// MARK: - Individual Subscriber Stats

#[derive(Debug, PartialEq, Eq, uniffi::Record)]
pub struct IndividualSubscriberStatsParams {
    pub subscription_id: SubscriptionId,
}

impl AppendUrlQueryPairs for IndividualSubscriberStatsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_query_value_pair("subscription_id", &self.subscription_id);
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct IndividualSubscriberStats {
    emails_sent: u64,
    unique_opens: u64,
    unique_clicks: u64,
    blog_registration_date: String,
}

// MARK: - Add Subscribers

#[derive(Debug, Serialize, Deserialize, Default, uniffi::Record)]
pub struct AddSubscribersParams {
    // A list of emails to add as subscribers to the current site.
    pub emails: Vec<String>,
    // A list of category IDs the emails should be subscribed to.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    // If true, the import will only parse the file and return the number of subscribers that would be imported.
    #[uniffi(default = false)]
    #[serde(skip_serializing_if = "<&bool>::not")]
    pub parse_only: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct AddSubscribersResponse {
    pub upload_id: u64,
}

// MARK: - List Subscriber Import Jobs

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SubscriberImportJobStatus {
    // We added the emails.
    Pending,
    // Enqueued but the import job hasn't picked it up yet.
    Awaiting,
    // Import complete and successful.
    Imported,
    // Job started.
    Importing,
    // Import failed.
    Failed,
    // Job cancelled.
    Cancelled,
    // Handles future status codes that the server might return.
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(SubscriberImportJobStatus);

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SubscriberImportJob {
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub id: u64,
    pub categories: Vec<String>,
    pub status: SubscriberImportJobStatus,
    pub timestamp: WpGmtDateTime,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub email_count: u64,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub subscribed_count: u64,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub already_subscribed_count: u64,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub failed_subscribed_count: u64,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub paid_subscribers_count: u64,
    pub platform: Option<String>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub paid_subscribed_count: u64,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub paid_already_subscribed_count: u64,
    #[serde(default, deserialize_with = "deserialize_u64_or_string")]
    pub paid_failed_subscribed_count: u64,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SubscriberImportJobsListParams {
    #[uniffi(default = None)]
    pub status: Option<SubscriberImportJobStatus>,
}

impl AppendUrlQueryPairs for SubscriberImportJobsListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        if let Some(status) = &self.status {
            query_pairs_mut.append_pair("status", &status.to_string());
        }
    }
}

impl_as_query_value_for_new_type!(SubscriptionId);
uniffi::custom_newtype!(SubscriptionId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionId(pub u64);

impl std::str::FromStr for SubscriptionId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_for_new_type!(UploadId);
uniffi::custom_newtype!(UploadId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadId(pub u64);

impl std::str::FromStr for UploadId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for UploadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// MARK: - Subscriber Stats

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SubscriberStatsResponse {
    pub counts: SubscriberCounts,
    pub aggregate: HashMap<String, SubscriberSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SubscriberCounts {
    pub email_subscribers: u64,
    pub all_subscribers: u64,
    pub paid_subscribers: u64,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct SubscriberSnapshot {
    pub all: u64,
    pub paid: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_error::WpError;

    #[test]
    fn test_list_subscribers_parameters_serialization() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers")
                .expect("Failed to parse url");

        let params = SubscribersListParams {
            page: Some(1),
            per_page: Some(100),
            search: Some("test".to_string()),
            sort: Some(ListSubscribersSortField::DateSubscribed),
            sort_order: Some(WpApiParamOrder::Asc),
            filter: Some(SubscriberType::All),
            filters: Some(vec![SubscriberType::All]),
            include: vec![],
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers?page=1&per_page=100&search=test&sort=date_subscribed&sort_order=asc&filters=all"
        );
    }

    #[test]
    fn test_list_subscribers_parameters_serialization_with_filter() {
        let mut url =
            url::Url::parse("https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers")
                .expect("Failed to parse url");

        let params = SubscribersListParams {
            page: Some(1),
            per_page: Some(100),
            search: Some("test".to_string()),
            sort: Some(ListSubscribersSortField::DateSubscribed),
            sort_order: Some(WpApiParamOrder::Asc),
            filter: Some(SubscriberType::EmailSubscriber),
            filters: None,
            include: vec![],
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers?page=1&per_page=100&search=test&sort=date_subscribed&sort_order=asc&filter=email_subscriber"
        );
    }

    #[test]
    fn test_subscriber_list_response_deserialization() {
        let json_file_path = "tests/wpcom/subscribers/subscriber-list.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: ListSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.total, 4);
        assert_eq!(response.pages, 1);
        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 100);
        assert_eq!(response.subscribers.len(), 4);
    }

    #[test]
    fn test_subscriber_list_with_invalid_date_returns_parsing_error() {
        let json_file_path = "tests/wpcom/subscribers/subscriber-list-with-invalid-date.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let result: Result<ListSubscribersResponse, _> = serde_json::from_reader(file);
        assert!(result.is_err(), "Expected parsing error for malformed date");
    }

    #[test]
    fn test_subscriber_list_not_allowed_response() {
        let json_file_path = "tests/wpcom/subscribers/subscriber-list-not-authorized.json";
        let file: std::fs::File = std::fs::File::open(json_file_path).expect("Failed to open file");
        let wp_error = WpError::try_parse_from_file(file).expect("Failed to parse JSON");
        assert_eq!(
            wp_error.message,
            "Only users with the permission to edit posts can access this endpoint."
        );
    }

    #[test]
    fn test_get_subscriber_query_serialization_for_wpcom_subscription() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers/individual",
        )
        .expect("Failed to parse url");

        let mut query_pairs = url.query_pairs_mut();
        IndividualSubscriberParams::WpCom(UserId(123)).append_query_pairs(&mut query_pairs);
        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers/individual?user_id=123&type=wpcom"
        );
    }

    #[test]
    fn test_get_subscriber_query_serialization_for_email_subscription() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers/individual",
        )
        .expect("Failed to parse url");

        let mut query_pairs = url.query_pairs_mut();
        IndividualSubscriberParams::Email(SubscriptionId(123)).append_query_pairs(&mut query_pairs);
        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers/individual?subscription_id=123&type=email"
        );
    }

    #[test]
    fn test_add_subscriber_serialization() {
        let params = AddSubscribersParams {
            emails: vec!["test@test.com".to_string()],
            categories: Some(vec!["123".to_string()]),
            parse_only: true,
        };

        let serialized = serde_json::to_string(&params).unwrap();
        assert_eq!(
            serialized,
            r#"{"emails":["test@test.com"],"categories":["123"],"parse_only":true}"#
        );
    }

    #[test]
    fn test_add_subscriber_response_deserialization() {
        let json_file_path = "tests/wpcom/subscribers/add-subscriber-response.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: AddSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(response.upload_id, 147487442134);
    }

    #[test]
    fn test_parse_only_is_not_serialized_when_false() {
        let params = AddSubscribersParams {
            emails: vec!["test@test.com".to_string()],
            categories: Some(vec!["123".to_string()]),
            parse_only: false,
        };
        let serialized = serde_json::to_string(&params).unwrap();
        assert_eq!(
            serialized,
            r#"{"emails":["test@test.com"],"categories":["123"]}"#
        );
    }

    #[test]
    fn test_categories_is_not_serialized_when_none() {
        let params = AddSubscribersParams {
            emails: vec!["test@test.com".to_string()],
            categories: None,
            parse_only: true,
        };
        let serialized = serde_json::to_string(&params).unwrap();
        assert_eq!(
            serialized,
            r#"{"emails":["test@test.com"],"parse_only":true}"#
        );
    }

    #[test]
    fn test_list_subscriber_import_jobs_parameters_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers/import",
        )
        .expect("Failed to parse url");

        let mut query_pairs = url.query_pairs_mut();
        SubscriberImportJobsListParams {
            status: Some(SubscriberImportJobStatus::Pending),
        }
        .append_query_pairs(&mut query_pairs);
        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers/import?status=pending"
        );
    }

    #[test]
    fn test_subscriber_import_job_list_response_deserialization() {
        let json_file_path = "tests/wpcom/subscribers/subscribers-import-jobs-list.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: Vec<SubscriberImportJob> =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(response.len(), 2);
    }

    #[test]
    fn test_subscriber_stats_response_deserialization() {
        let json_file_path = "tests/wpcom/subscribers/subscriber-stats.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: SubscriberStatsResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(response.counts.email_subscribers, 26);
        assert_eq!(response.aggregate.len(), 60);
    }

    #[test]
    fn test_get_subscriber_with_paid_plans_response_deserialization() {
        let json_file_path = "tests/wpcom/subscribers/subscriber-with-paid-plans.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let subscriber: Subscriber = serde_json::from_reader(file).expect("Unable to parse JSON");

        let plans = subscriber.plans.expect("JSON file includes plans");
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].start_date.to_string(), "2025-01-13T18:51:55+00:00");
    }

    #[test]
    fn test_individual_subscriber_stats_response_deserialization() {
        let json_file_path = "tests/wpcom/subscribers/individual-subscriber-stats.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let stats: IndividualSubscriberStats =
            serde_json::from_reader(file).expect("Unable to parse JSON");
        assert_eq!(stats.emails_sent, 2);
    }

    #[test]
    fn test_subscribers_by_user_type_params_serialization() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers_by_user_type",
        )
        .expect("Failed to parse url");

        let params = SubscribersByUserTypeParams {
            per_page: Some(10),
            page: Some(1),
            user_type: Some(SubscribersByUserTypeUserType::WpCom),
            sort: Some(SubscribersByUserTypeSortField::DateSubscribed),
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers_by_user_type?per_page=10&page=1&user_type=wpcom&sort=date_subscribed"
        );
    }

    #[test]
    fn test_subscribers_by_user_type_params_serialization_partial() {
        let mut url = url::Url::parse(
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers_by_user_type",
        )
        .expect("Failed to parse url");

        let params = SubscribersByUserTypeParams {
            per_page: Some(20),
            page: None,
            user_type: Some(SubscribersByUserTypeUserType::Email),
            sort: None,
        };

        let mut query_pairs = url.query_pairs_mut();
        params.append_query_pairs(&mut query_pairs);

        assert_eq!(
            query_pairs.finish().as_str(),
            "https://public-api.wordpress.com/wpcom/v2/sites/1234/subscribers_by_user_type?per_page=20&user_type=email"
        );
    }

    #[test]
    fn test_subscribers_by_user_type_response_deserialization() {
        let json_file_path = "tests/wpcom/subscribers/subscribers-by-user-type.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: ListSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.total, 88);
        assert_eq!(response.pages, 9);
        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 10);
        assert!(!response.is_owner_subscribed);
        assert_eq!(response.subscribers.len(), 3);

        let first = &response.subscribers[0];
        assert_eq!(first.user_id, UserId(33840434));
        assert_eq!(first.subscription_id, SubscriptionId(792200219));
        assert_eq!(first.display_name, "Nik");
        assert!(!first.is_email_subscriber);
        assert!(first.subscription_status.is_none());
        assert_eq!(first.url, Some("https://nikhilc.dev".to_string()));
    }

    #[test]
    fn test_subscribers_by_user_type_with_invalid_date_returns_parsing_error() {
        let json_file_path =
            "tests/wpcom/subscribers/subscribers-by-user-type-with-invalid-date.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let result: Result<ListSubscribersResponse, _> = serde_json::from_reader(file);
        assert!(result.is_err(), "Expected parsing error for malformed date");
    }

    #[test]
    fn test_subscribers_by_user_type_empty_response() {
        let json_file_path = "tests/wpcom/subscribers/subscribers-by-user-type-empty.json";
        let file = std::fs::File::open(json_file_path).expect("Failed to open file");
        let response: ListSubscribersResponse =
            serde_json::from_reader(file).expect("Unable to parse JSON");

        assert_eq!(response.total, 0);
        assert!(response.subscribers.is_empty());
    }
}
