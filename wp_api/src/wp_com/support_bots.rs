use crate::{
    date::WpGmtDateTime,
    impl_as_query_value_for_new_type,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    users::UserId,
};
use serde::{Deserialize, Serialize};
use serde_repr::*;
use std::collections::HashMap;
use wp_serde_helper::ok_or_default;

use super::WpComSiteId;

#[derive(Debug, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct CreateBotConversationParams {
    pub message: String,
    pub user_id: UserId,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct BotConversation {
    pub chat_id: u64,
    pub wpcom_user_id: UserId,
    pub external_id: Option<String>,
    pub external_id_provider: Option<String>,
    pub session_id: String,
    pub bot_slug: String,
    pub bot_version: String,
    pub created_at: WpGmtDateTime,
    pub zendesk_ticket_id: Option<String>,
    pub messages: Vec<BotMessage>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct BotMessage {
    pub message_id: u64,
    pub content: String,
    pub role: String,
    #[serde(rename = "ts")]
    pub created_at: WpGmtDateTime,
    pub context: MessageContext,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged, rename_all = "lowercase")]
pub enum MessageContext {
    User(UserMessageContext),
    Bot(BotMessageContext),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct BotMessageContext {
    pub question_tags: HashMap<String, String>,
    pub sources: Vec<BotMessageContextSource>,
    pub flags: HashMap<String, bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct BotMessageContextSource {
    pub title: String,
    pub url: String,
    pub heading: String,
    pub content: String,
    pub blog_id: WpComSiteId,
    pub post_id: u64,
    pub score: f64,
    pub last_indexed_at: WpGmtDateTime,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "snake_case")]
pub struct UserMessageContext {
    #[serde(alias = "selectedSiteId")]
    pub selected_site_id: Option<WpComSiteId>,
    pub wpcom_user_id: UserId,
    pub wpcom_user_name: String,
    pub user_paid_support_eligibility: UserPaidSupportEligibility,
    #[serde(deserialize_with = "ok_or_default")]
    pub plan: Option<UserPaidSupportPlan>,
    pub products: Vec<String>,
    pub plan_interface: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct UserPaidSupportEligibility {
    pub is_user_eligible: bool,
    pub wapuu_assistant_enabled: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct UserPaidSupportPlan {
    pub plan_name: String,
    pub is_free: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct BotConversationSummary {
    pub chat_id: u64,
    pub created_at: WpGmtDateTime,
    pub last_message: BotMessageSummary,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct BotMessageSummary {
    pub content: String,
    pub role: String,
    pub created_at: WpGmtDateTime,
}

#[derive(Debug, PartialEq, Eq, Deserialize, uniffi::Record)]
pub struct GetBotConversationParams {
    // The number of the page to retrieve, limited to 100.
    #[uniffi(default = None)]
    pub page_number: Option<u64>,

    // The number of items per page.
    #[uniffi(default = None)]
    pub items_per_page: Option<u64>,

    // If true, include the feedback rating value for each message in the response.
    #[uniffi(default = false)]
    pub include_feedback: bool,
}

impl AppendUrlQueryPairs for GetBotConversationParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("page", self.page_number.as_ref())
            .append_option_query_value_pair("per_page", self.items_per_page.as_ref())
            .append_query_value_pair("include_feedback", &self.include_feedback.to_string());
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct AddMessageToBotConversationParams {
    pub message: String,
    pub context: HashMap<String, String>, // TODO: Once it's possible, this hashmap should default to empty
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct CreateBotConversationFeedbackParams {
    pub rating_value: FeedbackRating,
}

#[derive(Debug, PartialEq, Eq, uniffi::Enum, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FeedbackRating {
    Positive = 1,
    Negative = 0,
}

impl_as_query_value_for_new_type!(BotId);
uniffi::custom_newtype!(BotId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BotId(pub String);

impl std::fmt::Display for BotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_for_new_type!(ChatId);
uniffi::custom_newtype!(ChatId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatId(pub u64);

impl std::str::FromStr for ChatId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for ChatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_for_new_type!(MessageId);
uniffi::custom_newtype!(MessageId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageId(pub u64);

impl std::str::FromStr for MessageId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::io::Read;

    use super::*;

    #[test]
    fn test_bot_create_conversation_response_deserialization() {
        let json = include_str!("../../tests/wpcom/support_bots/create-conversation-response.json");
        let conversation: BotConversation =
            serde_json::from_str(json).expect("Failed to deserialize bot conversation");
        assert_eq!(conversation.chat_id, 1965886);
    }

    #[rstest]
    #[case("single-conversation-01.json", 1965758)]
    #[case("single-conversation-02.json", 2826307)]
    fn test_bot_conversation_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_chat_id: u64,
    ) {
        let json = test_json(json_file_path).expect("Failed to read JSON file");
        let conversation: BotConversation = serde_json::from_slice(json.as_slice())
            .expect("Failed to deserialize bot conversation");
        assert_eq!(conversation.chat_id, expected_chat_id);
    }

    #[test]
    fn test_bot_conversation_summary_deserialization() {
        let json = include_str!("../../tests/wpcom/support_bots/converation-list.json");
        let conversations: Vec<BotConversationSummary> =
            serde_json::from_str(json).expect("Failed to deserialize bot conversation summary");
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].chat_id, 1965758);
    }

    #[test]
    fn test_add_message_to_bot_conversation_response_deserialization() {
        let json = include_str!(
            "../../tests/wpcom/support_bots/add-message-to-conversation-response.json"
        );
        let conversation: BotConversation =
            serde_json::from_str(json).expect("Failed to deserialize bot conversation");
        assert_eq!(conversation.chat_id, 1965758);
    }

    fn test_json(input: &str) -> Result<Vec<u8>, std::io::Error> {
        let mut file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        file_path.push("wp_api");
        file_path.push("tests");
        file_path.push("wpcom");
        file_path.push("support_bots");
        file_path.push(input);

        let mut f = std::fs::File::open(file_path)?;
        let mut buffer = Vec::new();

        // read the whole file
        f.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}
