use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    JsonValue,
    date::WpGmtDateTime,
    request::{MultipartFormFile, RequiresMultipartForm},
    wp_com::support_tickets::ConversationId,
};

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
    #[uniffi(default = [])]
    pub encrypted_log_ids: Vec<String>,
    #[serde(skip)]
    #[uniffi(default = [])]
    pub attachments: Vec<String>,
}

impl RequiresMultipartForm for ReplyToUnifiedConversationParams {
    fn multipart_form_files(&self) -> HashMap<String, MultipartFormFile> {
        self.attachments
            .iter()
            .enumerate()
            .map(|(i, file_path)| {
                (
                    format!("attachment_{i}"),
                    MultipartFormFile {
                        file_path: file_path.clone(),
                        mime_type: None,
                        file_name: None,
                    },
                )
            })
            .collect()
    }
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
        // Replies return the full conversation, including the newly posted
        // message as the last entry.
        let json = include_str!("../../tests/wpcom/unified_conversations/reply-response.json");
        let conversation: UnifiedConversation =
            serde_json::from_str(json).expect("Failed to deserialize reply response");
        assert_eq!(conversation.id, ConversationId(4396575));
        assert_eq!(conversation.messages.len(), 11);
        let reply = conversation.messages.last().unwrap();
        assert_eq!(reply.id, 17670102);
        assert_eq!(reply.author_role, "user");
        assert!(reply.attachments.is_empty());
    }

    #[test]
    fn test_reply_params_multipart_form_files() {
        // Each attachment file path is uploaded as a `attachment_{i}` file part,
        // verbatim (no MIME / filename overrides).
        let params = ReplyToUnifiedConversationParams {
            message: "Thanks!".to_string(),
            encrypted_log_ids: vec![],
            attachments: vec![
                "/tmp/screenshot-1.png".to_string(),
                "/tmp/screenshot-2.png".to_string(),
            ],
        };

        let files = params.multipart_form_files();
        assert_eq!(files.len(), 2);
        assert_eq!(files["attachment_0"].file_path, "/tmp/screenshot-1.png");
        assert_eq!(files["attachment_1"].file_path, "/tmp/screenshot-2.png");
        assert!(files["attachment_0"].mime_type.is_none());
        assert!(files["attachment_0"].file_name.is_none());
    }

    #[test]
    fn test_reply_params_multipart_text_fields() {
        use crate::request::WpMultipartFormField;

        let params = ReplyToUnifiedConversationParams {
            message: "Thanks!".to_string(),
            encrypted_log_ids: vec!["log-a".to_string(), "log-b".to_string()],
            attachments: vec!["/tmp/screenshot.png".to_string()],
        };

        // Mirror `request.rs::post_multipart`: serialize the params to JSON, then
        // expand each key into multipart text fields.
        let value = serde_json::to_value(&params).expect("params serialize");
        let object = value
            .as_object()
            .expect("params serialize to a JSON object");

        // `attachments` is `#[serde(skip)]` — it must NOT appear as a text field;
        // it is uploaded as a file part instead (see the test above).
        assert!(!object.contains_key("attachments"));

        let mut text_fields: Vec<(String, String)> = object
            .iter()
            .flat_map(|(key, value)| WpMultipartFormField::from_json(key.clone(), value.clone()))
            .map(|field| match field {
                WpMultipartFormField::Text { name, value } => (name, value),
                WpMultipartFormField::File { name, .. } => {
                    panic!("unexpected file field from serialized params: {name}")
                }
            })
            .collect();
        text_fields.sort();

        // `encrypted_log_ids` is sent as indexed form fields, one per id.
        assert_eq!(
            text_fields,
            vec![
                ("encrypted_log_ids[0]".to_string(), "log-a".to_string()),
                ("encrypted_log_ids[1]".to_string(), "log-b".to_string()),
                ("message".to_string(), "Thanks!".to_string()),
            ]
        );
    }
}
