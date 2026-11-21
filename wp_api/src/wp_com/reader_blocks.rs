//! Blocked sites: sites whose posts the user doesn't want the Reader to show.
//!
//! A block belongs to the account. The Reader's streams leave out a blocked site's posts.

use crate::{
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::{WpComSiteId, reader_saved::deserialize_optional_text},
};
use serde::{Deserialize, Serialize};
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::deserialize_u64_or_string_or_none;

/// The answer to blocking or unblocking a site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderBlockSiteResponse {
    /// Whether the block changed. `false` when the site was already in the state asked for, such
    /// as unblocking a site that isn't blocked, which is still a 200.
    pub success: bool,
}

/// Query parameters for the list of blocked sites.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The page number is a plain parameter: the answer has no paging headers to follow.
pub struct ReaderBlockedSitesParams {
    /// The page to return, from 1.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// How many sites a page has. The API's default is 100.
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
}

/// A page of the sites the user blocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderBlockedSitesResponse {
    pub sites: Vec<ReaderBlockedSite>,
    /// The page this is, from 1.
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    #[uniffi(default = None)]
    pub page: Option<u64>,
    /// How many sites this page has.
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    #[uniffi(default = None)]
    pub count: Option<u64>,
}

/// A site the user blocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderBlockedSite {
    #[serde(rename = "ID")]
    pub id: WpComSiteId,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_the_answer_to_a_block() {
        let changed: ReaderBlockSiteResponse =
            serde_json::from_value(json!({ "success": true, "modified_sub": true })).unwrap();
        let unchanged: ReaderBlockSiteResponse =
            serde_json::from_value(json!({ "success": false, "modified_sub": false })).unwrap();
        assert!(changed.success);
        assert!(!unchanged.success);
    }

    #[test]
    fn test_the_blocked_sites() {
        let page: ReaderBlockedSitesResponse = serde_json::from_value(json!({
            "sites": [{ "ID": 123, "name": "A site", "URL": "https://example.com" }],
            "page": 1,
            "count": 1,
        }))
        .unwrap();
        assert_eq!(page.sites[0].id, WpComSiteId(123));
        assert_eq!(page.sites[0].name.as_deref(), Some("A site"));
        assert_eq!(page.count, Some(1));

        let none: ReaderBlockedSitesResponse =
            serde_json::from_value(json!({ "sites": [], "page": 1, "count": 0 })).unwrap();
        assert!(none.sites.is_empty());
    }
}
