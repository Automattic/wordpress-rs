use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{JsonValue, date::WpGmtDateTime, wp_com::support_tickets::ConversationId};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct UnifiedConversationSummary {
    pub id: ConversationId,
    pub title: String,
    pub description: String,
    pub status: String,
    pub can_accept_reply: bool,
    pub created_at: WpGmtDateTime,
    pub updated_at: WpGmtDateTime,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct UnifiedConversation {
    pub id: ConversationId,
    pub title: String,
    pub description: String,
    pub status: String,
    pub can_accept_reply: bool,
    pub created_at: WpGmtDateTime,
    pub updated_at: WpGmtDateTime,
    pub messages: Vec<UnifiedMessage>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct UnifiedMessage {
    pub id: u64,
    pub message: String,
    pub author_role: String,
    pub author_name: String,
    pub created_at: WpGmtDateTime,
    pub attachments: Vec<UnifiedAttachment>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct UnifiedAttachment {
    pub id: u64,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub url: String,
    pub metadata: HashMap<String, JsonValue>,
}

#[derive(Debug, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReplyToUnifiedConversationParams {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_conversation_list_deserialization() {
        let json = include_str!("../../tests/wpcom/unified_conversations/conversation-list.json");
        let list: Vec<UnifiedConversationSummary> =
            serde_json::from_str(json).expect("Failed to deserialize unified conversation list");
        assert_eq!(list.len(), 70);
        // Spot-check the first entry (a conversation that can still accept replies).
        assert_eq!(list[0].id, ConversationId(11121776));
        assert!(list[0].can_accept_reply);
        assert_eq!(list[0].status, "new");
    }

    #[test]
    fn test_unified_conversation_deserialization() {
        let json = include_str!("../../tests/wpcom/unified_conversations/single-conversation.json");
        let conversation: UnifiedConversation =
            serde_json::from_str(json).expect("Failed to deserialize unified conversation");
        assert_eq!(conversation.id, ConversationId(4396575));
        assert_eq!(conversation.messages.len(), 10);
        assert_eq!(conversation.messages[0].author_role, "user");
        assert_eq!(conversation.messages[1].author_role, "bot");
        // Bot responses include reference attachments with a float `score` in metadata.
        let bot_attachments = &conversation.messages[7].attachments;
        assert!(!bot_attachments.is_empty());
        assert!(matches!(
            bot_attachments[0].metadata.get("score"),
            Some(JsonValue::Float(_))
        ));
    }

    #[test]
    fn test_unified_conversation_reply_response_deserialization() {
        // Replies return the full conversation, so the reply-response fixture
        // uses the same shape as the single-conversation fixture.
        let json = include_str!("../../tests/wpcom/unified_conversations/reply-response.json");
        let conversation: UnifiedConversation =
            serde_json::from_str(json).expect("Failed to deserialize reply response");
        assert_eq!(conversation.id, ConversationId(4396575));
        assert!(!conversation.messages.is_empty());
    }
}
