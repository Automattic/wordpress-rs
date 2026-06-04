use crate::impl_as_query_value_for_new_type;
use serde::{Deserialize, Serialize};

impl_as_query_value_for_new_type!(PublicizeConnectionId);
uniffi::custom_newtype!(PublicizeConnectionId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublicizeConnectionId(pub String);

impl std::fmt::Display for PublicizeConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A social media connection from the site-level publicize connections endpoint.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct PublicizeConnectionResponse {
    pub connection_id: String,
    pub display_name: String,
    /// Nullable per the server schema (e.g. when the service does not expose a
    /// public handle). Modelled as `Option<String>` so a `null` response
    /// deserializes cleanly.
    pub external_handle: Option<String>,
    pub external_id: String,
    pub profile_link: String,
    pub profile_picture: String,
    pub service_label: String,
    pub service_name: String,
    pub shared: bool,
    pub wpcom_user_id: i64,
    pub id: String,
    pub username: String,
    pub profile_display_name: String,
    pub global: bool,
    pub status: Option<String>,
}

/// An available social media service from the publicize services endpoint.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct PublicizeServiceResponse {
    pub id: String,
    pub description: String,
    pub label: String,
    pub status: String,
    pub supports: PublicizeServiceSupports,
    pub url: String,
}

/// Capabilities of a publicize service.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct PublicizeServiceSupports {
    pub additional_users: bool,
    pub additional_users_only: bool,
}

/// Parameters for creating a new publicize connection.
#[derive(Debug, Serialize, uniffi::Record)]
pub struct CreatePublicizeConnectionParams {
    #[serde(rename = "keyring_connection_ID")]
    pub keyring_connection_id: i64,
    #[serde(rename = "external_user_ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<bool>,
}

/// Parameters for updating an existing publicize connection.
///
/// The Jetpack publicize REST controller's `EDITABLE` route only accepts
/// `shared`; other fields (including `external_user_ID`) are ignored
/// server-side, so they're omitted here to keep the API honest.
#[derive(Debug, Serialize, uniffi::Record)]
pub struct UpdatePublicizeConnectionParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_connections_response() {
        let json = include_str!("../../tests/wpcom/publicize/connections.json");
        let connections: Vec<PublicizeConnectionResponse> =
            serde_json::from_str(json).expect("Failed to deserialize connections");
        assert_eq!(connections.len(), 2);
        assert_eq!(connections[0].connection_id, "25868837");
        assert_eq!(connections[0].display_name, "@tonya8c@mastodon.social");
        assert_eq!(
            connections[0].external_handle,
            Some("@tonya8c@mastodon.social".to_string())
        );
        assert_eq!(connections[0].service_name, "mastodon");
        assert!(connections[0].shared);
        assert_eq!(connections[0].status, None);
        assert_eq!(connections[1].connection_id, "25868865");
        assert_eq!(connections[1].service_name, "bluesky");
        assert_eq!(connections[1].status, Some("broken".to_string()));
    }

    #[test]
    fn test_deserialize_connection_with_null_external_handle() {
        // Server schema declares `external_handle` as ["string", "null"].
        // Confirm a null value lands as `None` rather than failing.
        let json = r#"{
            "connection_id": "1", "display_name": "Some Account",
            "external_handle": null, "external_id": "abc",
            "profile_link": "", "profile_picture": "",
            "service_label": "Service", "service_name": "service",
            "shared": false, "wpcom_user_id": 0, "id": "1",
            "username": "", "profile_display_name": "",
            "global": false, "status": null
        }"#;
        let connection: PublicizeConnectionResponse =
            serde_json::from_str(json).expect("Failed to deserialize null external_handle");
        assert_eq!(connection.external_handle, None);
    }

    #[test]
    fn test_deserialize_services_response() {
        let json = include_str!("../../tests/wpcom/publicize/services.json");
        let services: Vec<PublicizeServiceResponse> =
            serde_json::from_str(json).expect("Failed to deserialize services");
        assert_eq!(services.len(), 3);
        assert_eq!(services[0].id, "bluesky");
        assert_eq!(services[0].label, "Bluesky");
        assert!(!services[0].supports.additional_users);
        assert_eq!(services[1].id, "facebook");
        assert!(services[1].supports.additional_users_only);
    }

    #[test]
    fn test_deserialize_empty_connections() {
        let connections: Vec<PublicizeConnectionResponse> =
            serde_json::from_str("[]").expect("Failed to deserialize empty connections");
        assert!(connections.is_empty());
    }

    #[test]
    fn test_deserialize_empty_services() {
        let services: Vec<PublicizeServiceResponse> =
            serde_json::from_str("[]").expect("Failed to deserialize empty services");
        assert!(services.is_empty());
    }

    #[test]
    fn test_serialize_create_connection_params() {
        let params = CreatePublicizeConnectionParams {
            keyring_connection_id: 12345,
            external_user_id: Some("67890".to_string()),
            shared: Some(true),
        };
        let json = serde_json::to_value(&params).expect("Failed to serialize");
        assert_eq!(json["keyring_connection_ID"], 12345);
        assert_eq!(json["external_user_ID"], "67890");
        assert_eq!(json["shared"], true);
    }

    #[test]
    fn test_serialize_create_connection_params_minimal() {
        let params = CreatePublicizeConnectionParams {
            keyring_connection_id: 12345,
            external_user_id: None,
            shared: None,
        };
        let json = serde_json::to_value(&params).expect("Failed to serialize");
        assert_eq!(json["keyring_connection_ID"], 12345);
        assert!(json.get("external_user_ID").is_none());
        assert!(json.get("shared").is_none());
    }

    #[test]
    fn test_serialize_update_connection_params() {
        let params = UpdatePublicizeConnectionParams {
            shared: Some(false),
        };
        let json = serde_json::to_value(&params).expect("Failed to serialize");
        assert_eq!(json["shared"], false);
    }

    #[test]
    fn test_serialize_update_connection_params_minimal() {
        let params = UpdatePublicizeConnectionParams { shared: None };
        let json = serde_json::to_value(&params).expect("Failed to serialize");
        assert!(json.get("shared").is_none());
    }
}
