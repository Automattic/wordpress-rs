use std::fmt::Display;

use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct SiteSettingsUpdateParams {
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_format: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_of_week: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_smilies: Option<bool>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_category: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_post_format: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts_per_page: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_front: Option<String>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_on_front: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_for_posts: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_ping_status: Option<SiteSettingsPingStatus>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_comment_status: Option<SiteSettingsCommentStatus>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_logo: Option<u64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_icon: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseSiteSettings {
    #[WpContext(edit, embed, view)]
    pub title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub url: Option<String>,
    #[WpContext(edit, embed, view)]
    pub email: Option<String>,
    #[WpContext(edit, embed, view)]
    pub timezone: Option<String>,
    #[WpContext(edit, embed, view)]
    pub date_format: Option<String>,
    #[WpContext(edit, embed, view)]
    pub time_format: Option<String>,
    #[WpContext(edit, embed, view)]
    pub start_of_week: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub language: Option<String>,
    #[WpContext(edit, embed, view)]
    pub use_smilies: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub default_category: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub default_post_format: Option<String>,
    #[WpContext(edit, embed, view)]
    pub posts_per_page: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub show_on_front: Option<String>,
    #[WpContext(edit, embed, view)]
    pub page_on_front: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub page_for_posts: Option<u64>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub default_ping_status: Option<SiteSettingsPingStatus>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub default_comment_status: Option<SiteSettingsCommentStatus>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub site_logo: Option<u64>,
    #[WpContext(edit, embed, view)]
    pub site_icon: Option<u64>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum SiteSettingsPingStatus {
    Open,
    Closed,
    #[serde(untagged)]
    Custom(String),
}

impl Display for SiteSettingsPingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Custom(name) => name.as_str(),
        };
        write!(f, "{s}")
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum SiteSettingsCommentStatus {
    Open,
    Closed,
    #[serde(untagged)]
    Custom(String),
}

impl Display for SiteSettingsCommentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Custom(name) => name.as_str(),
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    // Reproduces the API response from the error report.
    // `default_ping_status` is null, which previously caused a parsing failure
    // because serde's untagged enum variant prevented Option from handling null.
    const RESPONSE_WITH_NULL_PING_STATUS: &str = r#"{
        "title": "Test Site",
        "description": "",
        "url": "https://example.com",
        "email": "test@example.com",
        "timezone": "",
        "date_format": "F j, Y",
        "time_format": "g:i a",
        "start_of_week": 1,
        "language": "en_US",
        "use_smilies": true,
        "default_category": 1,
        "default_post_format": "aside",
        "posts_per_page": 10,
        "show_on_front": "page",
        "page_on_front": 10,
        "page_for_posts": 0,
        "default_ping_status": null,
        "default_comment_status": "open",
        "site_logo": null,
        "site_icon": 0
    }"#;

    fn json_with_statuses(ping: &str, comment: &str) -> String {
        RESPONSE_WITH_NULL_PING_STATUS
            .replace(
                r#""default_ping_status": null"#,
                &format!(r#""default_ping_status": {ping}"#),
            )
            .replace(
                r#""default_comment_status": "open""#,
                &format!(r#""default_comment_status": {comment}"#),
            )
    }

    // Test all three generated contextual types with null status values,
    // since WpContextual macro strips the Option wrapper unless
    // #[WpContextualOption] is present.
    #[test]
    fn test_parse_view_context_with_null_ping_status() {
        let settings: SiteSettingsWithViewContext =
            serde_json::from_str(RESPONSE_WITH_NULL_PING_STATUS).unwrap();
        assert_eq!(settings.default_ping_status, None);
        assert_eq!(
            settings.default_comment_status,
            Some(SiteSettingsCommentStatus::Open)
        );
    }

    #[test]
    fn test_parse_edit_context_with_null_ping_status() {
        let settings: SiteSettingsWithEditContext =
            serde_json::from_str(RESPONSE_WITH_NULL_PING_STATUS).unwrap();
        assert_eq!(settings.default_ping_status, None);
        assert_eq!(
            settings.default_comment_status,
            Some(SiteSettingsCommentStatus::Open)
        );
    }

    #[test]
    fn test_parse_embed_context_with_null_ping_status() {
        let settings: SiteSettingsWithEmbedContext =
            serde_json::from_str(RESPONSE_WITH_NULL_PING_STATUS).unwrap();
        assert_eq!(settings.default_ping_status, None);
        assert_eq!(
            settings.default_comment_status,
            Some(SiteSettingsCommentStatus::Open)
        );
    }

    #[test]
    fn test_parse_view_context_with_both_statuses_null() {
        let json = json_with_statuses("null", "null");
        let settings: SiteSettingsWithViewContext = serde_json::from_str(&json).unwrap();
        assert_eq!(settings.default_ping_status, None);
        assert_eq!(settings.default_comment_status, None);
    }

    #[rstest]
    #[case("\"open\"", Some(SiteSettingsPingStatus::Open))]
    #[case("\"closed\"", Some(SiteSettingsPingStatus::Closed))]
    #[case("\"custom_value\"", Some(SiteSettingsPingStatus::Custom("custom_value".to_string())))]
    #[case("null", None)]
    fn test_parse_view_context_ping_status_values(
        #[case] json_value: &str,
        #[case] expected: Option<SiteSettingsPingStatus>,
    ) {
        let json = json_with_statuses(json_value, "\"open\"");
        let settings: SiteSettingsWithViewContext = serde_json::from_str(&json).unwrap();
        assert_eq!(settings.default_ping_status, expected);
    }

    #[rstest]
    #[case("\"open\"", Some(SiteSettingsCommentStatus::Open))]
    #[case("\"closed\"", Some(SiteSettingsCommentStatus::Closed))]
    #[case("\"custom_value\"", Some(SiteSettingsCommentStatus::Custom("custom_value".to_string())))]
    #[case("null", None)]
    fn test_parse_view_context_comment_status_values(
        #[case] json_value: &str,
        #[case] expected: Option<SiteSettingsCommentStatus>,
    ) {
        let json = json_with_statuses("\"open\"", json_value);
        let settings: SiteSettingsWithViewContext = serde_json::from_str(&json).unwrap();
        assert_eq!(settings.default_comment_status, expected);
    }

    #[test]
    fn test_parse_sparse_with_missing_ping_status_field() {
        let json = r#"{
            "title": "Test Site",
            "description": "",
            "url": "https://example.com",
            "email": "test@example.com",
            "timezone": "",
            "date_format": "F j, Y",
            "time_format": "g:i a",
            "start_of_week": 1,
            "language": "en_US",
            "use_smilies": true,
            "default_category": 1,
            "default_post_format": "aside",
            "posts_per_page": 10,
            "show_on_front": "page",
            "page_on_front": 10,
            "page_for_posts": 0,
            "default_comment_status": "open",
            "site_logo": null,
            "site_icon": 0
        }"#;
        let settings: SparseSiteSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.default_ping_status, None);
    }
}
