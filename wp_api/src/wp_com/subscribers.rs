use crate::{
    WpApiParamOrder,
    date::WpGmtDateTime,
    impl_as_query_value_for_new_type, impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    users::UserId,
};

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Not};
use wp_serde_helper::deserialize_u64_or_string;

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct Subscriber {
    pub user_id: UserId,
    pub display_name: String,
    pub email_address: String,
    pub email_subscription_id: Option<u64>,
    pub date_subscribed: WpGmtDateTime,
    pub subscription_status: String,
    pub avatar: String,
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

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct SubscribersListParams {
    // The current page.
    #[uniffi(default = None)]
    pub page: Option<u64>,
    // The amount of items to show per page.
    #[uniffi(default = None)]
    pub per_page: Option<u64>,
    // Search for subscribers
    #[uniffi(default = None)]
    pub search: Option<String>,
    // Sort subscribers by a specific field
    #[uniffi(default = None)]
    pub sort: Option<ListSubscribersSortField>,
    // Sort order
    #[uniffi(default = None)]
    pub sort_order: Option<WpApiParamOrder>,
    // Filter subscribers by a specific subscriber type
    #[uniffi(default = None)]
    pub filter: Option<SubscriberType>,
    // Array of filters to apply (combined with AND logic). If provided, overrides the single filter parameter.
    #[uniffi(default = None)]
    pub filters: Option<Vec<SubscriberType>>,
}

impl AppendUrlQueryPairs for SubscribersListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("page", self.page.as_ref())
            .append_option_query_value_pair("per_page", self.per_page.as_ref())
            .append_option_query_value_pair("search", self.search.as_ref())
            .append_option_query_value_pair("sort", self.sort.as_ref())
            .append_option_query_value_pair("sort_order", self.sort_order.as_ref());

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
    EmailAddress,
    DisplayName,
    Plan,
    SubscriptionStatus,
}

impl_as_query_value_from_to_string!(ListSubscribersSortField);

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ListSubscribersResponse {
    pub total: u64,
    pub pages: u64,
    pub page: u64,
    pub per_page: u64,
    pub subscribers: Vec<Subscriber>,
}

// MARK: - Get Subscriber

#[derive(Debug, uniffi::Enum)]
pub enum GetSubscriberQuery {
    // Return subscribers that receive notifications via WordPress.com for new posts.
    WpCom(u64),

    // Return subscribers that receive notifications via email for new posts.
    Email(u64),
}

impl AppendUrlQueryPairs for GetSubscriberQuery {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        match self {
            GetSubscriberQuery::WpCom(user_id) => {
                query_pairs_mut.append_pair("user_id", &user_id.to_string());
                query_pairs_mut.append_pair("type", "wpcom");
            }
            GetSubscriberQuery::Email(email) => {
                query_pairs_mut.append_pair("subscription_id", &email.to_string());
                query_pairs_mut.append_pair("type", "email");
            }
        }
    }
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

        assert_eq!(response.total, 8);
        assert_eq!(response.pages, 1);
        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 100);
        assert_eq!(response.subscribers.len(), 8);
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
        GetSubscriberQuery::WpCom(123).append_query_pairs(&mut query_pairs);
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
        GetSubscriberQuery::Email(123).append_query_pairs(&mut query_pairs);
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
}
