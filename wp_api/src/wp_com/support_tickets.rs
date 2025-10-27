use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{date::WpGmtDateTime, impl_as_query_value_for_new_type};

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
    pub attachments: Vec<String>,
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
    pub metadata: HashMap<AttachmentMetadataKey, AttachmentMetadataValue>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SupportMessageAuthor {
    User(SupportUserIdentity),
    SupportAgent(SupportAgentIdentity),
}

#[derive(
    Debug, Hash, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum, strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
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

impl AttachmentMetadataValue {
    pub fn get_number(&self) -> Option<u64> {
        match self {
            AttachmentMetadataValue::Number(number) => Some(*number),
            _ => None,
        }
    }
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct AttachmentDimensions {
    pub width: u64,
    pub height: u64,
}

#[uniffi::export]
pub fn get_attachment_dimensions(attachment: &SupportAttachment) -> Option<AttachmentDimensions> {
    let metadata = &attachment.metadata;

    let width = metadata
        .get(&AttachmentMetadataKey::Width)
        .and_then(|v| v.get_number());
    let height = metadata
        .get(&AttachmentMetadataKey::Height)
        .and_then(|v| v.get_number());

    if let Some(width) = width
        && let Some(height) = height
    {
        return Some(AttachmentDimensions { width, height });
    }

    None
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

    #[test]
    fn test_support_conversation_with_attachments_deserialization() {
        let json = include_str!(
            "../../tests/wpcom/support_tickets/single-conversation-with-attachments.json"
        );
        let conversation: SupportConversation =
            serde_json::from_str(json).expect("Failed to deserialize support conversation");
        assert_eq!(conversation.messages.len(), 1);
        assert_eq!(conversation.messages[0].attachments.len(), 2);
        assert_eq!(
            conversation.messages[0].attachments[0].filename,
            "sample-image-1.jpg"
        );
        assert_eq!(
            conversation.messages[0].attachments[0].content_type,
            "image/jpeg"
        );
        assert_eq!(conversation.messages[0].attachments[0].size, 123456);
        assert_eq!(
            conversation.messages[0].attachments[0].url,
            "https://example.com/attachments/token/token1/?name=sample-image-1.jpg"
        );

        let dimensions =
            get_attachment_dimensions(&conversation.messages[0].attachments[0]).unwrap();

        assert_eq!(dimensions.width, 1000);
        assert_eq!(dimensions.height, 800);

        assert_eq!(
            conversation.messages[0].attachments[1].filename,
            "sample-image-2.jpg"
        );
        assert_eq!(
            conversation.messages[0].attachments[1].content_type,
            "image/jpeg"
        );
        assert_eq!(conversation.messages[0].attachments[1].size, 654321);
        assert_eq!(
            conversation.messages[0].attachments[1].url,
            "https://example.com/attachments/token/token2/?name=sample-image-2.jpg"
        );

        let dimensions =
            get_attachment_dimensions(&conversation.messages[0].attachments[1]).unwrap();
        assert_eq!(dimensions.width, 2000);
        assert_eq!(dimensions.height, 1600);
    }
}
