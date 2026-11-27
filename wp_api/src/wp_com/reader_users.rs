//! A WordPress.com user's public profile, as the Reader shows it.
//!
//! This is what anyone signed in can see of any user. It isn't the account's own details, which
//! `GET /me` returns as [`WPComUserInfo`](super::me::WPComUserInfo) with the user's email,
//! language and sites, and the two share only an ID, a display name and an avatar.

use crate::url_query::{
    AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
};
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
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::LenientVec;

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

/// Query parameters for the people of a site to suggest when a reader types `@` in a comment.
/// Name the site with `site_id` or `site`.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // It answers with every suggestion at once.
pub struct ReaderUserSuggestionsParams {
    /// The site whose people to suggest. The API answers 403 `invalid_query` with neither this
    /// nor `site`, and 403 `unauthorized` for a site that isn't there.
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// The site by its domain, instead of `site_id`, which it overrides.
    #[uniffi(default = None)]
    pub site: Option<String>,
    /// Only the people whose login or name matches it: on the WordPress.com Blog, `j` left 30 of
    /// its 175 and `ja` left 4, not all of them starting with it. Without it, all of them. Both
    /// WordPress apps fetch them all and filter as the reader types.
    #[uniffi(default = None)]
    pub filter: Option<String>,
    /// The size of the avatars in [`ReaderUserSuggestion::image_url`], in pixels: 96 by default,
    /// between 1 and 2048. Not every avatar comes at it: on the WordPress.com Blog, 21 of 175 came
    /// at 32 whatever was asked (2026-11-21).
    #[uniffi(default = None)]
    pub image_size: Option<u32>,
    /// A name for the app asking. It changed nothing in the answers that were looked at.
    #[uniffi(default = None)]
    pub client: Option<String>,
}

/// The people of a site to suggest when a reader types `@` in a comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderUserSuggestionsResponse")]
pub struct ReaderUserSuggestionsResponse {
    /// Ordered as the server sent them, without any in [`Self::unreadable_suggestions`].
    pub suggestions: Vec<ReaderUserSuggestion>,
    /// Why each suggestion that couldn't be read couldn't be. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_suggestions: Vec<String>,
}

#[derive(Deserialize)]
struct RawReaderUserSuggestionsResponse {
    #[serde(default)]
    suggestions: Option<LenientVec<ReaderUserSuggestion>>,
}

impl From<RawReaderUserSuggestionsResponse> for ReaderUserSuggestionsResponse {
    fn from(raw: RawReaderUserSuggestionsResponse) -> Self {
        let suggestions = raw.suggestions.unwrap_or_default();
        Self {
            unreadable_suggestions: suggestions
                .invalid
                .into_iter()
                .map(|suggestion| suggestion.error.to_string())
                .collect(),
            suggestions: suggestions.valid,
        }
    }
}

/// Someone to suggest when a reader types `@` in a comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderUserSuggestion {
    /// What goes after the `@`.
    pub user_login: String,
    #[serde(
        rename = "ID",
        default,
        deserialize_with = "deserialize_optional_nonzero_id"
    )]
    #[uniffi(default = None)]
    pub user_id: Option<WpComUserId>,
    /// Can carry character references such as `&amp;`. Decode it with
    /// `decode_html_entities` to show it as text.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub display_name: Option<String>,
    /// The person's avatar, at the size asked for with
    /// [`ReaderUserSuggestionsParams::image_size`].
    #[serde(
        rename = "image_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub image_url: Option<String>,
    /// Why the person is suggested. Every one seen was `site`: a user of the site.
    #[serde(default)]
    #[uniffi(default = [])]
    pub source: Vec<String>,
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
    fn test_suggestions() {
        let json = include_str!("../../tests/wpcom/reader_users/suggest.json");
        let response: ReaderUserSuggestionsResponse =
            serde_json::from_str(json).expect("the suggestions parse");

        assert_eq!(response.suggestions.len(), 2);
        assert!(response.unreadable_suggestions.is_empty());
        let first = &response.suggestions[0];
        assert_eq!(first.user_login, "exampleauthor");
        assert_eq!(first.user_id, Some(WpComUserId(100000001)));
        assert_eq!(first.display_name.as_deref(), Some("Example Author"));
        assert!(first.image_url.as_deref().unwrap().contains("s=96"));
        assert_eq!(first.source, ["site"]);
        assert_eq!(
            response.suggestions[1].display_name.as_deref(),
            Some("Jane &amp; Co")
        );
    }

    /// Signed out, and for a site with no one to suggest, the list is empty. One suggestion that
    /// can't be read doesn't fail the others.
    #[test]
    fn test_no_suggestions_and_one_that_cant_be_read() {
        let empty: ReaderUserSuggestionsResponse =
            serde_json::from_value(json!({ "suggestions": [] })).expect("an empty list parses");
        assert!(empty.suggestions.is_empty());

        let mixed: ReaderUserSuggestionsResponse = serde_json::from_value(json!({
            "suggestions": [
                { "ID": "100000003", "user_login": "someone", "display_name": "" },
                { "ID": 100000004, "display_name": "No login" },
            ],
        }))
        .expect("the suggestions parse");
        assert_eq!(mixed.suggestions.len(), 1);
        assert_eq!(mixed.suggestions[0].user_id, Some(WpComUserId(100000003)));
        assert_eq!(mixed.suggestions[0].display_name, None);
        assert!(mixed.suggestions[0].source.is_empty());
        assert_eq!(mixed.unreadable_suggestions.len(), 1);
    }

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
