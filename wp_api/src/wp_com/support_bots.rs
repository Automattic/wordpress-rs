use crate::{
    date::WpGmtDateTime,
    url_query::{AppendUrlQueryPairs, QueryPairs, QueryPairsExtension},
    users::UserId,
};
use serde::{Deserialize, Serialize};
use serde_repr::*;
use std::collections::HashMap;

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
    pub selected_site_id: WpComSiteId,
    pub wpcom_user_id: UserId,
    pub wpcom_user_name: String,
    pub user_paid_support_eligibility: UserPaidSupportEligibility,
    pub plan: UserPaidSupportPlan,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_create_conversation_response_deserialization() {
        let json = include_str!("../../tests/wpcom/support_bots/create-conversation-response.json");
        let conversation: BotConversation =
            serde_json::from_str(json).expect("Failed to deserialize bot conversation");
        assert_eq!(conversation.chat_id, 1965886);
    }

    #[test]
    fn test_bot_conversation_deserialization() {
        let json = include_str!("../../tests/wpcom/support_bots/single-conversation.json");
        let conversation: BotConversation =
            serde_json::from_str(json).expect("Failed to deserialize bot conversation");
        assert_eq!(conversation.chat_id, 1965758);
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
}
