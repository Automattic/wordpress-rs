//! A WordPress.com user's public profile, as the Reader shows it.
//!
//! This is what anyone signed in can see of any user. It isn't the account's own details, which
//! `GET /me` returns as [`WPComUserInfo`](super::me::WPComUserInfo) with the user's email,
//! language and sites, and the two share only an ID, a display name and an avatar.

use crate::wp_com::{
    WpComSiteId,
    me::WpComUserId,
    reader_saved::{ReaderFeedId, deserialize_optional_text},
    reader_sites::{
        deserialize_object_or_none, deserialize_optional_count, deserialize_optional_nonzero_id,
    },
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies a WordPress.com user, by login or by ID.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum WpComUserIdentifier {
    /// The user's login, in upper or lower case.
    Login { value: String },
    /// The user's ID.
    ///
    /// The API reads the number as a login first. A few users have a login that is a number, so
    /// the ID of one user can find another: `5` finds the user whose login is `5`, not user 5.
    /// Check [`ReaderUserProfile::id`] against the ID asked for, and prefer the login when it's
    /// known.
    Id { value: WpComUserId },
}

impl fmt::Display for WpComUserIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Login { value } => write!(f, "{value}"),
            Self::Id { value } => write!(f, "{value}"),
        }
    }
}

/// A WordPress.com user's public profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderUserProfile {
    #[serde(
        rename = "ID",
        deserialize_with = "wp_serde_helper::deserialize_u64_or_string_as_t"
    )]
    pub id: WpComUserId,
    #[serde(
        rename = "user_login",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub login: Option<String>,
    /// Can carry character references such as `&amp;`. Decode it with
    /// `decode_html_entities` to show it as text.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub display_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub first_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub last_name: Option<String>,
    /// The login as it appears in an address.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub nice_name: Option<String>,
    /// What the user wrote about themselves. It can have line breaks, and character references
    /// such as `&amp;`. `None` for the many users who wrote nothing.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub description: Option<String>,
    #[serde(
        rename = "avatar_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub avatar_url: Option<String>,
    /// The user's Gravatar profile.
    #[serde(
        rename = "profile_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub profile_url: Option<String>,
    /// The user's main site. `None` for a user with no site.
    #[serde(
        rename = "primary_blog",
        default,
        deserialize_with = "deserialize_object_or_none"
    )]
    #[uniffi(default = None)]
    pub primary_site: Option<ReaderUserPrimarySite>,
    /// How many sites the user recommends. `0` for most users.
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    #[uniffi(default = None)]
    pub recommended_blogs_count: Option<u64>,
}

/// The main site of the user in a [`ReaderUserProfile`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderUserPrimarySite {
    #[serde(
        rename = "ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// The site's feed. `None` when the site has no feed, for which the API sends `0`.
    #[serde(
        rename = "feed_ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// Can carry character references such as `&amp;`. Decode it with
    /// `decode_html_entities` to show it as text.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub title: Option<String>,
    /// Can carry character references such as `&amp;`. Decode it with
    /// `decode_html_entities` to show it as text.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub description: Option<String>,
    /// The site's icon. About half of all sites have none, for which the API sends `null`.
    #[serde(
        rename = "avatar_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub icon_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    /// A recording of a user's public profile, with the user's details replaced.
    #[test]
    fn test_deserialize_profile() {
        let json = include_str!("../../tests/wpcom/reader_users/profile.json");
        let profile: ReaderUserProfile = serde_json::from_str(json).expect("the profile parses");

        assert_eq!(profile.id, WpComUserId(5001));
        assert_eq!(profile.login.as_deref(), Some("example"));
        assert_eq!(profile.display_name.as_deref(), Some("Example Name"));
        assert_eq!(profile.first_name.as_deref(), Some("Example"));
        assert_eq!(profile.last_name.as_deref(), Some("Name"));
        assert_eq!(profile.nice_name.as_deref(), Some("example"));
        assert_eq!(
            profile.description.as_deref(),
            Some("An example description.")
        );
        assert_eq!(
            profile.avatar_url.as_deref(),
            Some("https://example.com/avatar.png")
        );
        assert!(profile.profile_url.is_some());
        assert_eq!(profile.recommended_blogs_count, Some(0));
        assert_eq!(
            profile.primary_site,
            Some(ReaderUserPrimarySite {
                site_id: Some(WpComSiteId(2001)),
                feed_id: Some(ReaderFeedId(3001)),
                url: Some("https://example.wordpress.com".to_string()),
                title: Some("Example Site".to_string()),
                description: Some("An example site.".to_string()),
                icon_url: Some("https://example.com/icon.png".to_string()),
            })
        );
    }

    /// What the API sends for a user with no site, no names and nothing about themselves.
    #[test]
    fn test_a_profile_without_a_site() {
        let profile: ReaderUserProfile = serde_json::from_value(json!({
            "ID": 5001,
            "user_login": "5",
            "first_name": "",
            "last_name": "",
            "display_name": "5",
            "nice_name": "5",
            "description": "",
            "avatar_URL": "https://example.com/avatar.png",
            "profile_URL": "https://example.com/5",
            "primary_blog": null,
            "recommended_blogs_count": 0,
        }))
        .expect("the profile parses");

        assert_eq!(profile.login.as_deref(), Some("5"));
        assert_eq!(profile.first_name, None);
        assert_eq!(profile.last_name, None);
        assert_eq!(profile.description, None);
        assert_eq!(profile.primary_site, None);
    }

    /// A main site with no feed has a `feed_ID` of `0`, and one with no icon a `null` one.
    #[test]
    fn test_a_primary_site_without_a_feed_or_an_icon() {
        let profile: ReaderUserProfile = serde_json::from_value(json!({
            "ID": 5001,
            "primary_blog": {
                "ID": 2001,
                "feed_ID": 0,
                "URL": "https://example.wordpress.com",
                "title": "",
                "description": "",
                "avatar_URL": null,
            },
        }))
        .expect("the profile parses");

        let site = profile.primary_site.expect("the user has a site");
        assert_eq!(site.site_id, Some(WpComSiteId(2001)));
        assert_eq!(site.feed_id, None);
        assert_eq!(site.title, None);
        assert_eq!(site.icon_url, None);
    }

    #[test]
    fn test_a_profile_without_an_id_is_an_error() {
        let profile = json!({ "user_login": "example" });
        assert!(serde_json::from_value::<ReaderUserProfile>(profile).is_err());
    }

    #[rstest]
    #[case(WpComUserIdentifier::Login { value: "example".to_string() }, "example")]
    #[case(WpComUserIdentifier::Id { value: WpComUserId(5001) }, "5001")]
    fn test_user_identifier_as_text(#[case] user: WpComUserIdentifier, #[case] expected: &str) {
        assert_eq!(user.to_string(), expected);
    }
}
