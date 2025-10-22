use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    date::WpGmtDateTime, impl_as_query_value_for_new_type, request::RequiresMultipartForm,
};

#[derive(Debug, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct CreateSupportTicketParams {
    pub subject: String,
    pub message: String,
    pub application: String,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wpcom_site_id: Option<u64>,
    #[uniffi(default = [])]
    pub tags: Vec<String>,
    #[uniffi(default = [])]
    #[serde(skip)]
    pub attachments: Vec<String>,
}

impl RequiresMultipartForm for CreateSupportTicketParams {
    fn multipart_form_files(&self) -> Vec<String> {
        self.attachments.clone()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct SupportConversationSummary {
    pub id: ConversationId,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: WpGmtDateTime,
    pub updated_at: WpGmtDateTime,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct SupportConversation {
    pub id: ConversationId,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: WpGmtDateTime,
    pub updated_at: WpGmtDateTime,
    pub messages: Vec<SupportMessage>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct SupportMessage {
    pub id: u64,
    pub content: String,
    pub author: SupportMessageAuthor,
    pub role: String,
    pub author_is_current_user: bool,
    pub created_at: WpGmtDateTime,
    pub attachments: Vec<SupportAttachment>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct SupportUserIdentity {
    pub id: u64,
    pub email: String,
    pub display_name: String,
    pub avatar_url: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct SupportAttachment {
    pub id: u64,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub url: String,
    pub metadata: HashMap<String, AttachmentMetadataValue>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SupportMessageAuthor {
    User(SupportUserIdentity),
    SupportAgent(SupportAgentIdentity),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum AttachmentMetadataKey {
    Width,
    Height,
    Other(String),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum AttachmentMetadataValue {
    String(String),
    Number(u64),
    Boolean(bool),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct SupportAgentIdentity {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct AddMessageToSupportConversationParams {
    pub message: String,
    #[uniffi(default = [])]
    pub attachments: Vec<String>,
}

impl_as_query_value_for_new_type!(ConversationId);
uniffi::custom_newtype!(ConversationId, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationId(pub u64);

impl std::str::FromStr for ConversationId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_support_conversation_deserialization() {
        let json = include_str!("../../tests/wpcom/support_tickets/single-conversation.json");
        let conversation: SupportConversation =
            serde_json::from_str(json).expect("Failed to deserialize support conversation");
        assert_eq!(conversation.messages.len(), 7);
    }

    #[test]
    fn test_support_conversation_list_deserialization() {
        let json = include_str!("../../tests/wpcom/support_tickets/conversation-list.json");
        let conversation_list: Vec<SupportConversationSummary> =
            serde_json::from_str(json).expect("Failed to deserialize support conversation list");
        assert_eq!(conversation_list.len(), 11);
    }
}
