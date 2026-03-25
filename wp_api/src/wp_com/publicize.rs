use serde::{Deserialize, Serialize};

/// A social media connection from the site-level publicize connections endpoint.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct PublicizeConnectionResponse {
    pub connection_id: String,
    pub display_name: String,
    pub external_handle: String,
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
        assert_eq!(connections[0].service_name, "mastodon");
        assert!(connections[0].shared);
        assert_eq!(connections[0].status, None);
        assert_eq!(connections[1].connection_id, "25868865");
        assert_eq!(connections[1].service_name, "bluesky");
        assert_eq!(connections[1].status, Some("broken".to_string()));
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
}
