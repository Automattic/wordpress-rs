use crate::wp_com::domains::CountryCode;
use crate::{JsonValue, date::WpGmtDateTime, wp_content_u64_id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_serde_helper::{
    deserialize_empty_string_as_none, deserialize_null_as_empty_vec,
    deserialize_string_vec_or_string, deserialize_u64_or_none,
    deserialize_u64_or_none_with_negative_as_none, deserialize_u64_or_none_with_zero_as_none,
};

wp_content_u64_id!(WpComUserId);

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPComUserInfo {
    /// The user's WP.com ID.
    #[serde(rename = "ID")]
    pub id: u64,

    /// The user's display name as set in the `Public display name` field
    pub display_name: String,

    /// The user's username as set at account creation. This cannot be changed.
    pub username: String,

    /// The user's email address.
    pub email: String,

    /// The user's primary blog ID – this is the one that was created when they made their account.
    #[serde(rename = "primary_blog")]
    #[serde(deserialize_with = "deserialize_u64_or_none")]
    pub primary_blog_id: Option<u64>,

    /// The user's primary blog URL – this is the one that was created when they made their account.
    pub primary_blog_url: Option<String>,

    /// Whether the user's primary blog is a Jetpack blog.
    pub primary_blog_is_jetpack: bool,

    /// Whether the user has Jetpack partner access.
    pub has_jetpack_partner_access: bool,

    /// The partner types of the partner accounts this user has access to.
    #[serde(default)]
    pub jetpack_partner_types: Vec<String>,

    /// The user's preferred language.
    pub language: String,

    /// The variant of the user's preferred language.
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    pub locale_variant: Option<String>,

    /// If the current access token is scoped to a specific Site ID, this field will be set to that Site ID. Otherwise, it will be null.
    #[serde(deserialize_with = "deserialize_u64_or_none_with_zero_as_none")]
    pub token_site_id: Option<u64>,

    /// The scopes of the current access token – see https://developer.wordpress.com/docs/api/oauth2/ for a list of possible values.
    #[serde(rename = "token_scope")]
    #[serde(deserialize_with = "deserialize_string_vec_or_string")]
    pub token_scopes: Vec<String>,

    /// If the current access token is scoped to a specific Client ID, this field will be set to that Client ID. Otherwise, it will be null.
    #[serde(deserialize_with = "deserialize_u64_or_none_with_negative_as_none")]
    pub token_client_id: Option<u64>,

    /// The user's avatar URL as set on WordPress.com or using Gravatar.
    #[serde(rename = "avatar_URL")]
    pub avatar_url: Option<String>,

    #[serde(rename = "profile_URL")]
    /// The user's Gravatar profile URL.
    pub profile_url: Option<String>,

    /// Whether the user's email address has been verified via WordPress.com Connect.
    pub verified: bool,

    /// Whether the user's email address has been verified – their ability to perform many actions requires this to be true.
    pub email_verified: bool,

    /// The date of the user's account creation.
    #[serde(rename = "date")]
    pub creation_date: WpGmtDateTime,

    /// The number of sites the user has access to.
    pub site_count: u64,

    /// The number of sites the user has access to that are Jetpack sites.
    pub jetpack_site_count: u64,

    /// The number of sites the user has access to that are Atomic sites.
    pub atomic_site_count: u64,

    /// The number of sites the user has access to that are visible.
    pub visible_site_count: u64,

    /// The number of visible sites the user has access to that are Jetpack sites.
    pub jetpack_visible_site_count: u64,

    /// The number of visible sites the user has access to that are Atomic sites.
    pub atomic_visible_site_count: u64,

    /// Whether the user has unseen notifications.
    pub has_unseen_notes: bool,

    /// The type of the user's newest notification.
    pub newest_note_type: Option<String>,

    /// If this is a phone account then the user doesn't have a verified email address
    pub phone_account: bool,

    /// Is the user somewhere where Google Workspace can be purchased?
    pub is_valid_google_apps_country: bool,

    /// Country code for the user's IP address.
    pub user_ip_country_code: Option<String>,

    /// Active social login connections.
    #[serde(deserialize_with = "deserialize_null_as_empty_vec")]
    pub social_login_connections: Vec<WpComSocialLoginConnection>,

    /// The name of the social service this account is linked to.
    pub social_signup_service: Option<String>,

    /// User's assigned A/B test variations, where the key is the test name and the value is the variation
    pub abtests: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpComSocialLoginConnection {
    pub service: String,
    pub service_user_email: String,
    pub service_user_id: String,
}

/// WHOIS/domain contact information for the authenticated user.
///
/// Returned by `GET /rest/v1.1/me/domain-contact-information/` and also
/// used as the request body for `POST /me/domain-contact-information/`
/// and as part of `POST /me/transactions/`.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct DomainContactInformation {
    /// First name.
    pub first_name: Option<String>,
    /// Last name.
    pub last_name: Option<String>,
    /// Organization or company name.
    pub organization: Option<String>,
    /// Address line 1.
    pub address_1: Option<String>,
    /// Address line 2.
    pub address_2: Option<String>,
    /// Postal or ZIP code.
    pub postal_code: Option<String>,
    /// City.
    pub city: Option<String>,
    /// State or province (code like `"MA"` or full name, depending on country).
    pub state: Option<String>,
    /// ISO 3166-1 alpha-2 country code.
    pub country_code: Option<CountryCode>,
    /// Contact email address.
    pub email: Option<String>,
    /// Phone number in RFC 5733 format (e.g. `"+1.5551234567"`).
    pub phone: Option<String>,
    /// Fax number in RFC 5733 format (legacy, usually `null`).
    pub fax: Option<String>,
    /// TLD-specific additional contact information (e.g. `.de` requirements).
    pub extra: Option<HashMap<String, JsonValue>>,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::io::Read;

    use super::*;

    #[rstest]
    #[case("v1.1-me-01.json", 742098)]
    #[case("v1.1-me-02.json", 158350866)]
    // IDs for this test aren't anonymized so that they can be checked later if needed.
    fn test_wpcom_user_info_deserialization(
        #[case] json_file_path: &str,
        #[case] expected_id: u64,
    ) {
        let json = test_json(json_file_path).expect("Failed to read JSON file");
        let user_info: WPComUserInfo =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize user info");
        assert_eq!(user_info.id, expected_id);
        assert!(user_info.avatar_url.is_some());
        assert!(user_info.profile_url.is_some());
    }

    #[rstest]
    #[case("domain-contact-information.json")]
    #[case("domain-contact-information-minimal.json")]
    #[case("domain-contact-information-with-extra.json")]
    fn test_domain_contact_information_deserialization(#[case] json_file_path: &str) {
        let json = test_json(json_file_path).expect("Failed to read JSON file");
        let _contact: DomainContactInformation =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize");
    }

    #[test]
    fn test_domain_contact_information_full() {
        let json = test_json("domain-contact-information.json").expect("Failed to read JSON file");
        let contact: DomainContactInformation =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize");

        assert_eq!(contact.first_name.as_deref(), Some("Jane"));
        assert_eq!(contact.last_name.as_deref(), Some("Smith"));
        assert_eq!(contact.organization.as_deref(), Some("Fake Corp"));
        assert_eq!(contact.address_1.as_deref(), Some("742 Evergreen Terrace"));
        assert_eq!(contact.address_2.as_deref(), Some("Suite 200"));
        assert_eq!(contact.postal_code.as_deref(), Some("90210"));
        assert_eq!(contact.city.as_deref(), Some("Springfield"));
        assert_eq!(contact.state.as_deref(), Some("IL"));
        assert_eq!(contact.country_code, Some(CountryCode("US".to_string())));
        assert_eq!(contact.email.as_deref(), Some("jane@fakecorp.example"));
        assert_eq!(contact.phone.as_deref(), Some("+1.5559876543"));
        assert!(contact.fax.is_none());
        assert!(contact.extra.is_none());
    }

    #[test]
    fn test_domain_contact_information_minimal() {
        let json =
            test_json("domain-contact-information-minimal.json").expect("Failed to read JSON file");
        let contact: DomainContactInformation =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize");

        assert_eq!(contact.first_name.as_deref(), Some("John"));
        assert_eq!(contact.last_name.as_deref(), Some("Doe"));
        assert!(contact.organization.is_none());
        assert!(contact.address_1.is_none());
        assert!(contact.country_code.is_none());
        assert_eq!(contact.email.as_deref(), Some("john@example.com"));
        assert!(contact.phone.is_none());
    }

    #[test]
    fn test_domain_contact_information_with_extra() {
        let json = test_json("domain-contact-information-with-extra.json")
            .expect("Failed to read JSON file");
        let contact: DomainContactInformation =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize");

        assert_eq!(contact.country_code, Some(CountryCode("DE".to_string())));
        let extra = contact.extra.expect("expected extra");
        assert!(extra.contains_key("de"));
    }

    #[test]
    fn test_domain_contact_information_serialization_roundtrip() {
        let json = test_json("domain-contact-information.json").expect("Failed to read JSON file");
        let contact: DomainContactInformation =
            serde_json::from_slice(json.as_slice()).expect("Failed to deserialize");
        let serialized = serde_json::to_string(&contact).expect("Failed to serialize");
        let deserialized: DomainContactInformation =
            serde_json::from_str(&serialized).expect("Failed to re-deserialize");
        assert_eq!(contact.first_name, deserialized.first_name);
        assert_eq!(contact.country_code, deserialized.country_code);
        assert_eq!(contact.phone, deserialized.phone);
    }

    fn test_json(input: &str) -> Result<Vec<u8>, std::io::Error> {
        let mut file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        file_path.push("wp_api");
        file_path.push("tests");
        file_path.push("wpcom");
        file_path.push("me");
        file_path.push(input);

        let mut f = std::fs::File::open(file_path)?;
        let mut buffer = Vec::new();

        // read the whole file
        f.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}
