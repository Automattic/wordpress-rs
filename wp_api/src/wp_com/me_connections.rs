use serde::{Deserialize, Serialize};

/// Envelope for the list endpoint: `{ "connections": [...] }`.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct MeConnectionsResponse {
    pub connections: Vec<KeyringConnectionResponse>,
}

/// A keyring connection (OAuth token for a third-party service).
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct KeyringConnectionResponse {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "user_ID")]
    pub user_id: i64,
    pub service: String,
    pub label: Option<String>,
    #[serde(rename = "external_ID")]
    pub external_id: String,
    pub external_name: String,
    pub external_display: String,
    pub external_profile_picture: Option<String>,
    pub status: String,
    #[serde(rename = "refresh_URL")]
    pub refresh_url: String,
    /// Defaults to an empty list when the field is omitted; some keyring
    /// services don't include it in the response at all.
    #[serde(default)]
    pub additional_external_users: Vec<KeyringExternalUser>,
}

/// An alternative account available on the same keyring connection.
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct KeyringExternalUser {
    #[serde(rename = "external_ID")]
    pub external_id: String,
    pub external_name: String,
    pub external_profile_picture: Option<String>,
    pub external_description: Option<String>,
    pub external_category: Option<String>,
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
    }

    #[test]
    fn test_deserialize_list_response_nullable_fields() {
        let json = include_str!("../../tests/wpcom/me_connections/list.json");
        let response: MeConnectionsResponse =
            serde_json::from_str(json).expect("Failed to deserialize");

        let facebook = &response.connections[1];
        assert_eq!(facebook.label, None);
        assert_eq!(facebook.external_profile_picture, None);
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
        assert_eq!(page.external_category, Some("Internet company".to_string()));

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
        assert_eq!(
            connection.refresh_url,
            "https://public-api.wordpress.com/connect/?action=request&service=mastodon"
        );
    }

    #[test]
    fn test_deserialize_empty_connections_list() {
        let json = r#"{"connections": []}"#;
        let response: MeConnectionsResponse =
            serde_json::from_str(json).expect("Failed to deserialize empty list");
        assert!(response.connections.is_empty());
    }

    #[test]
    fn test_deserialize_with_missing_additional_external_users() {
        // `additional_external_users` defaults to an empty list when the field
        // is omitted entirely (some services don't include it at all).
        let json = r#"{
            "ID": 1, "user_ID": 1, "service": "mastodon", "label": "Mastodon",
            "external_ID": "abc", "external_name": "handle",
            "external_display": "", "external_profile_picture": null,
            "status": "ok", "refresh_URL": ""
        }"#;
        let connection: KeyringConnectionResponse =
            serde_json::from_str(json).expect("Failed to deserialize without the field");
        assert!(connection.additional_external_users.is_empty());
    }
}
