use crate::AnyJson;
use crate::posts::PostMeta;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A social media connection from the jetpack_publicize_connections response field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct JetpackPublicizeConnection {
    /// Always present in current Jetpack versions.
    /// The deprecated `id` field is intentionally not modeled.
    pub connection_id: String,
    pub display_name: String,
    pub service_name: String,
    /// Whether this connection is enabled for sharing on this post.
    /// Only present in edit context.
    pub enabled: Option<bool>,
    /// Connection health status. None means healthy.
    /// Known values: "ok", "broken", "must_reauth".
    pub status: Option<String>,
}

/// Request payload for updating a connection's enabled state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct JetpackPublicizeConnectionUpdate {
    pub connection_id: String,
    pub enabled: bool,
}

/// Parse publicize connections from a post's additional_fields.
#[uniffi::export]
pub fn jetpack_social_publicize_connections(
    additional_fields: &AnyJson,
) -> Option<Vec<JetpackPublicizeConnection>> {
    let value = additional_fields.raw.get("jetpack_publicize_connections")?;
    serde_json::from_value(value.clone()).ok()
}

/// Parse the publicize message from post meta.
#[uniffi::export]
pub fn jetpack_social_publicize_message(meta: &PostMeta) -> Option<String> {
    meta.raw_value("jetpack_publicize_message")
        .and_then(|v| v.as_str().map(String::from))
}

/// Parse the publicize feature enabled flag from post meta.
/// This is the master toggle for sharing. Defaults to true on the server.
#[uniffi::export]
pub fn jetpack_social_publicize_feature_enabled(meta: &PostMeta) -> Option<bool> {
    meta.raw_value("jetpack_publicize_feature_enabled")
        .and_then(|v| v.as_bool())
}

/// Parse whether the post has already been shared from post meta.
/// This is a read-only server-set value.
#[uniffi::export]
pub fn jetpack_social_post_already_shared(meta: &PostMeta) -> Option<bool> {
    meta.raw_value("jetpack_social_post_already_shared")
        .and_then(|v| v.as_bool())
}

/// Insert/update publicize feature enabled flag into a PostMeta.
/// Preserves existing keys. Creates a new PostMeta if existing is None.
#[uniffi::export]
pub fn jetpack_social_set_publicize_feature_enabled(
    existing: Option<Arc<PostMeta>>,
    enabled: bool,
) -> Arc<PostMeta> {
    let base = existing.unwrap_or_else(PostMeta::empty);
    let json_value = serde_json::to_string(&enabled).expect("bool serialization should not fail");
    base.upsert("jetpack_publicize_feature_enabled".into(), json_value)
}

/// Insert/update publicize connection updates into an AnyJson.
/// Preserves existing keys. Creates a new AnyJson if existing is None.
#[uniffi::export]
pub fn jetpack_social_set_publicize_connections(
    existing: Option<Arc<AnyJson>>,
    connections: Vec<JetpackPublicizeConnectionUpdate>,
) -> Arc<AnyJson> {
    let base = existing.unwrap_or_else(AnyJson::empty);
    let json_value = serde_json::to_string(&connections)
        .expect("JetpackPublicizeConnectionUpdate serialization should not fail");
    base.upsert("jetpack_publicize_connections".into(), json_value)
}

/// Insert/update publicize message into a PostMeta.
/// Preserves existing keys. Creates a new PostMeta if existing is None.
#[uniffi::export]
pub fn jetpack_social_set_publicize_message(
    existing: Option<Arc<PostMeta>>,
    message: String,
) -> Arc<PostMeta> {
    let base = existing.unwrap_or_else(PostMeta::empty);
    let json_value = serde_json::to_string(&message).expect("String serialization should not fail");
    base.upsert("jetpack_publicize_message".into(), json_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_connections_json() -> String {
        r#"{"jetpack_publicize_connections": [
            {"connection_id": "123", "display_name": "My Page", "service_name": "facebook", "enabled": true, "status": null},
            {"connection_id": "456", "display_name": "@alice", "service_name": "mastodon", "enabled": false, "status": "broken"}
        ]}"#
        .to_string()
    }

    #[test]
    fn test_parse_connections_from_additional_fields() {
        let json = AnyJson::from_raw_json(sample_connections_json());
        let connections = jetpack_social_publicize_connections(&json).unwrap();
        assert_eq!(connections.len(), 2);
        assert_eq!(connections[0].connection_id, "123");
        assert_eq!(connections[0].display_name, "My Page");
        assert_eq!(connections[0].service_name, "facebook");
        assert_eq!(connections[0].enabled, Some(true));
        assert_eq!(connections[0].status, None);
        assert_eq!(connections[1].connection_id, "456");
        assert_eq!(connections[1].enabled, Some(false));
        assert_eq!(connections[1].status, Some("broken".to_string()));
    }

    #[test]
    fn test_parse_connections_absent() {
        let json = AnyJson::from_raw_json(r#"{"other": "data"}"#.to_string());
        assert_eq!(jetpack_social_publicize_connections(&json), None);
    }

    #[test]
    fn test_parse_connections_enabled_absent() {
        let json = AnyJson::from_raw_json(
            r#"{"jetpack_publicize_connections": [{"connection_id": "1", "display_name": "X", "service_name": "x"}]}"#.to_string(),
        );
        let connections = jetpack_social_publicize_connections(&json).unwrap();
        assert_eq!(connections[0].enabled, None);
    }

    #[test]
    fn test_parse_message_from_meta() {
        let meta: PostMeta =
            serde_json::from_str(r#"{"jetpack_publicize_message": "Check this out!"}"#).unwrap();
        assert_eq!(
            jetpack_social_publicize_message(&meta),
            Some("Check this out!".to_string())
        );
    }

    #[test]
    fn test_parse_message_absent() {
        let meta: PostMeta = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(jetpack_social_publicize_message(&meta), None);
    }

    #[test]
    fn test_set_connections_into_empty() {
        let updates = vec![
            JetpackPublicizeConnectionUpdate {
                connection_id: "123".to_string(),
                enabled: true,
            },
            JetpackPublicizeConnectionUpdate {
                connection_id: "456".to_string(),
                enabled: false,
            },
        ];
        let result = jetpack_social_set_publicize_connections(None, updates);
        // Verify the JSON structure directly — JetpackPublicizeConnectionUpdate has different
        // fields than JetpackPublicizeConnection, so we can't round-trip through the read function.
        let serialized = serde_json::to_string(result.as_ref()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let arr = value
            .get("jetpack_publicize_connections")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].get("connection_id").unwrap().as_str(), Some("123"));
        assert_eq!(arr[0].get("enabled").unwrap().as_bool(), Some(true));
        assert_eq!(arr[1].get("connection_id").unwrap().as_str(), Some("456"));
        assert_eq!(arr[1].get("enabled").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn test_set_connections_preserves_existing_keys() {
        let existing = AnyJson::from_raw_json(r#"{"some_taxonomy": [1, 2, 3]}"#.to_string());
        let updates = vec![JetpackPublicizeConnectionUpdate {
            connection_id: "1".to_string(),
            enabled: true,
        }];
        let result = jetpack_social_set_publicize_connections(Some(existing), updates);
        // Verify the connections key was set
        let serialized = serde_json::to_string(result.as_ref()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert!(value.get("jetpack_publicize_connections").is_some());
        // Verify existing key is preserved
        assert!(value.get("some_taxonomy").is_some());
    }

    #[test]
    fn test_set_message_into_empty() {
        let result = jetpack_social_set_publicize_message(None, "Hello world".to_string());
        assert_eq!(
            jetpack_social_publicize_message(&result),
            Some("Hello world".to_string())
        );
    }

    #[test]
    fn test_set_message_preserves_existing_meta() {
        let existing: PostMeta =
            serde_json::from_str(r#"{"footnotes": "[{\"id\":\"1\",\"content\":\"fn\"}]"}"#)
                .unwrap();
        let result =
            jetpack_social_set_publicize_message(Some(Arc::new(existing)), "msg".to_string());
        assert_eq!(
            jetpack_social_publicize_message(&result),
            Some("msg".to_string())
        );
        assert!(result.footnotes().is_some());
    }

    #[test]
    fn test_parse_feature_enabled_true() {
        let meta: PostMeta =
            serde_json::from_str(r#"{"jetpack_publicize_feature_enabled": true}"#).unwrap();
        assert_eq!(jetpack_social_publicize_feature_enabled(&meta), Some(true));
    }

    #[test]
    fn test_parse_feature_enabled_false() {
        let meta: PostMeta =
            serde_json::from_str(r#"{"jetpack_publicize_feature_enabled": false}"#).unwrap();
        assert_eq!(jetpack_social_publicize_feature_enabled(&meta), Some(false));
    }

    #[test]
    fn test_parse_feature_enabled_absent() {
        let meta: PostMeta = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(jetpack_social_publicize_feature_enabled(&meta), None);
    }

    #[test]
    fn test_parse_post_already_shared_true() {
        let meta: PostMeta =
            serde_json::from_str(r#"{"jetpack_social_post_already_shared": true}"#).unwrap();
        assert_eq!(jetpack_social_post_already_shared(&meta), Some(true));
    }

    #[test]
    fn test_parse_post_already_shared_false() {
        let meta: PostMeta =
            serde_json::from_str(r#"{"jetpack_social_post_already_shared": false}"#).unwrap();
        assert_eq!(jetpack_social_post_already_shared(&meta), Some(false));
    }

    #[test]
    fn test_parse_post_already_shared_absent() {
        let meta: PostMeta = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(jetpack_social_post_already_shared(&meta), None);
    }

    #[test]
    fn test_set_feature_enabled_into_empty() {
        let result = jetpack_social_set_publicize_feature_enabled(None, true);
        assert_eq!(
            jetpack_social_publicize_feature_enabled(&result),
            Some(true)
        );
    }

    #[test]
    fn test_set_feature_enabled_preserves_existing_meta() {
        let existing: PostMeta = serde_json::from_str(
            r#"{"jetpack_publicize_message": "hello", "jetpack_social_post_already_shared": true}"#,
        )
        .unwrap();
        let result = jetpack_social_set_publicize_feature_enabled(Some(Arc::new(existing)), false);
        assert_eq!(
            jetpack_social_publicize_feature_enabled(&result),
            Some(false)
        );
        assert_eq!(
            jetpack_social_publicize_message(&result),
            Some("hello".to_string())
        );
        assert_eq!(jetpack_social_post_already_shared(&result), Some(true));
    }

    #[test]
    fn test_round_trip_connections() {
        // Verify that a full JetpackPublicizeConnection (from a response) can be serialized
        // into AnyJson and parsed back.
        let connections_json = sample_connections_json();
        let json = AnyJson::from_raw_json(connections_json);
        let serialized = serde_json::to_string(json.as_ref()).unwrap();
        let deserialized = AnyJson::from_raw_json(serialized);
        let parsed = jetpack_social_publicize_connections(&deserialized).unwrap();
        assert_eq!(parsed[0].connection_id, "123");
        assert_eq!(parsed[1].service_name, "mastodon");
    }
}
