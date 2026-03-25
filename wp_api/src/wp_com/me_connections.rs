use crate::impl_as_query_value_for_new_type;
use serde::{Deserialize, Serialize};

impl_as_query_value_for_new_type!(KeyringTokenId);
uniffi::custom_newtype!(KeyringTokenId, i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyringTokenId(pub i64);

impl std::str::FromStr for KeyringTokenId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for KeyringTokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Envelope for the list endpoint: `{ "connections": [...] }`.
#[derive(Debug, Deserialize, uniffi::Record)]
pub struct MeConnectionsResponse {
    pub connections: Vec<KeyringConnectionResponse>,
}

/// A keyring connection (OAuth token for a third-party service).
#[derive(Debug, Deserialize, uniffi::Record)]
pub struct KeyringConnectionResponse {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "user_ID")]
    pub user_id: i64,
    pub service: String,
    pub label: Option<String>,
    pub issued: Option<String>,
    pub expires: Option<String>,
    #[serde(rename = "external_ID")]
    pub external_id: String,
    pub external_name: String,
    pub external_display: String,
    pub external_profile_picture: Option<String>,
    pub status: String,
    #[serde(rename = "refresh_URL")]
    pub refresh_url: String,
    pub additional_external_users: Vec<KeyringExternalUser>,
}

/// An alternative account available on the same keyring connection.
#[derive(Debug, Deserialize, uniffi::Record)]
pub struct KeyringExternalUser {
    #[serde(rename = "external_ID")]
    pub external_id: String,
    pub external_name: String,
    pub external_profile_picture: Option<String>,
    pub external_description: Option<String>,
    pub external_category: Option<String>,
}

/// Response from `DELETE /me/connections/{token_id}`.
#[derive(Debug, Deserialize, uniffi::Record)]
pub struct KeyringConnectionDeleteResponse {
    #[serde(rename = "ID")]
    pub id: i64,
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_list_response() {
        let json = include_str!("../../tests/wpcom/me_connections/list.json");
        let response: MeConnectionsResponse =
            serde_json::from_str(json).expect("Failed to deserialize list response");
        assert_eq!(response.connections.len(), 2);

        let mastodon = &response.connections[0];
        assert_eq!(mastodon.id, 98765);
        assert_eq!(mastodon.user_id, 12345);
        assert_eq!(mastodon.service, "mastodon");
        assert_eq!(mastodon.label, Some("Mastodon".to_string()));
        assert_eq!(mastodon.status, "ok");
        assert!(mastodon.additional_external_users.is_empty());
        assert_eq!(mastodon.expires, None);
    }

    #[test]
    fn test_deserialize_list_response_nullable_fields() {
        let json = include_str!("../../tests/wpcom/me_connections/list.json");
        let response: MeConnectionsResponse =
            serde_json::from_str(json).expect("Failed to deserialize");

        let facebook = &response.connections[1];
        assert_eq!(facebook.label, None);
        assert_eq!(facebook.issued, None);
        assert_eq!(facebook.external_profile_picture, None);
        assert_eq!(facebook.expires, Some("2026-03-25 00:00:00".to_string()));
    }

    #[test]
    fn test_deserialize_additional_external_users() {
        let json = include_str!("../../tests/wpcom/me_connections/list.json");
        let response: MeConnectionsResponse =
            serde_json::from_str(json).expect("Failed to deserialize");

        let facebook = &response.connections[1];
        assert_eq!(facebook.additional_external_users.len(), 2);

        let page = &facebook.additional_external_users[0];
        assert_eq!(page.external_id, "200012345678");
        assert_eq!(page.external_name, "My Page");
        assert_eq!(
            page.external_profile_picture,
            Some("https://example.com/page-pic.jpg".to_string())
        );
        assert_eq!(
            page.external_description,
            Some("A test Facebook page".to_string())
        );
        assert_eq!(
            page.external_category,
            Some("Internet company".to_string())
        );

        let page2 = &facebook.additional_external_users[1];
        assert_eq!(page2.external_profile_picture, None);
        assert_eq!(page2.external_description, None);
        assert_eq!(page2.external_category, None);
    }

    #[test]
    fn test_deserialize_single_connection() {
        let json = include_str!("../../tests/wpcom/me_connections/single.json");
        let connection: KeyringConnectionResponse =
            serde_json::from_str(json).expect("Failed to deserialize single connection");
        assert_eq!(connection.id, 98765);
        assert_eq!(connection.service, "mastodon");
        assert_eq!(connection.refresh_url, "https://public-api.wordpress.com/connect/?action=request&service=mastodon");
    }

    #[test]
    fn test_deserialize_delete_response() {
        let json = r#"{"ID": 98765, "deleted": true}"#;
        let response: KeyringConnectionDeleteResponse =
            serde_json::from_str(json).expect("Failed to deserialize delete response");
        assert_eq!(response.id, 98765);
        assert!(response.deleted);
    }

    #[test]
    fn test_deserialize_empty_connections_list() {
        let json = r#"{"connections": []}"#;
        let response: MeConnectionsResponse =
            serde_json::from_str(json).expect("Failed to deserialize empty list");
        assert!(response.connections.is_empty());
    }
}
