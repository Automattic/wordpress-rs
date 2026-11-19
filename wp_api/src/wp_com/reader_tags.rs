//! The Reader menu, tags, and the interests a new reader picks from.
//!
//! All three answer without a token, so a signed-out reader has streams and tags to show. See
//! [`ReaderMenu`] for what a signed-out menu holds.
//!
//! The API sends a tag's slug percent-encoded when it isn't ASCII: `%e5%86%99%e7%9c%9f` for `写真`.
//! Every slug here is decoded as it's read, so it can go straight into a [`ReaderTagSlug`]. A
//! request escapes the slug it's given, and the encoded form, escaped again, names no tag.

use crate::{
    wp_com::{reader_posts::ReaderTagSlug, reader_saved::deserialize_optional_text},
    wp_content_u64_id,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, SeqAccess, Visitor},
};
use std::fmt;
use wp_serde_helper::{deserialize_u64_or_string_or_none, deserialize_u64_or_string_or_none_as_t};

wp_content_u64_id!(ReaderTagId);

/// The Reader menu, from `GET /rest/v1.3/read/menu`.
///
/// Signed out, the API still answers. [`Self::default_items`] then has only Discover,
/// [`Self::subscribed_items`] is empty, and [`Self::recommended_items`] is the same list of tags a
/// signed-in reader gets. Tags a signed-out reader follows aren't known to the API, so an app keeps
/// them itself.
///
/// Each section keeps the order the API sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderMenu")]
pub struct ReaderMenu {
    /// `default`: the Reader's own streams, then the organizations the user belongs to, then the
    /// user's lists.
    pub default_items: Vec<ReaderMenuItem>,
    /// `subscribed`: the tags the user follows.
    pub subscribed_items: Vec<ReaderMenuItem>,
    /// `recommended`: tags to suggest. It isn't personal: every reader gets the same ones, and a
    /// tag the user follows can be among them.
    pub recommended_items: Vec<ReaderMenuItem>,
    /// The entries that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_items: Vec<ReaderUnreadableMenuItem>,
}

/// An entry of the Reader menu.
///
/// Which fields are set depends on [`Self::kind`]. An entry of a kind this version doesn't know is
/// kept, as [`ReaderMenuItemKind::Other`], with whichever of these fields it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderMenuItem {
    /// The entry's name in its section. In `default` it's `following`, `discover`, `liked`, `a8c`,
    /// `p2`, or `list-` and a list's slug. In `subscribed` and `recommended` it's the tag's ID.
    pub key: String,
    pub kind: ReaderMenuItemKind,
    /// The name to show, in the request's language for the Reader's own streams. HTML: a tag's can
    /// carry character references, such as `Art &amp; Design`.
    #[uniffi(default = None)]
    pub title: Option<String>,
    /// The API address of the entry's posts. A list's owner and slug are only here.
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// A tag's slug, to pass as a [`ReaderTagSlug`], or an organization's: `a8c` or `p2`.
    #[uniffi(default = None)]
    pub slug: Option<String>,
    /// A tag's name in lower case. Only a tag has one.
    #[uniffi(default = None)]
    pub display_name: Option<String>,
    /// A tag's ID. Only a tag has one.
    #[uniffi(default = None)]
    pub tag_id: Option<ReaderTagId>,
    /// `1` for Automattic and `2` for P2. Only an organization has one.
    #[uniffi(default = None)]
    pub organization_id: Option<u64>,
}

/// What an entry of the Reader menu is, from its `type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Enum)]
pub enum ReaderMenuItemKind {
    /// One of the Reader's own streams, which the API sends without a `type`: `following`,
    /// `discover` and `liked`.
    Stream,
    /// A tag.
    Tag,
    /// One of the user's lists.
    List,
    /// The posts of an organization's sites: `a8c` for Automattic, and `p2` for the P2s the user
    /// follows. An Automattician's menu has both, and a signed-out menu has neither.
    Organization,
    /// A kind this version doesn't know, with the `type` the API sent.
    Other(String),
}

impl ReaderMenuItemKind {
    fn from_type(kind: Option<&str>) -> Self {
        match kind {
            None => Self::Stream,
            Some("tag") => Self::Tag,
            Some("list") => Self::List,
            Some("organization") => Self::Organization,
            Some(other) => Self::Other(other.to_string()),
        }
    }
}

/// A section of the Reader menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, uniffi::Enum)]
pub enum ReaderMenuSection {
    Default,
    Subscribed,
    Recommended,
}

/// An entry of the Reader menu that couldn't be read. It doesn't fail the menu: the other
/// entries are kept, and this says which one was left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnreadableMenuItem {
    pub section: ReaderMenuSection,
    /// The entry's name in its section, or its position when the section arrived as a list.
    pub key: String,
    /// Why it couldn't be read.
    pub error: String,
}

#[derive(Deserialize)]
struct RawReaderMenu {
    #[serde(default)]
    default: RawReaderMenuSection,
    #[serde(default)]
    subscribed: RawReaderMenuSection,
    #[serde(default)]
    recommended: RawReaderMenuSection,
}

impl From<RawReaderMenu> for ReaderMenu {
    fn from(raw: RawReaderMenu) -> Self {
        let mut unreadable_items = Vec::new();
        let mut items = |section: ReaderMenuSection, raw: RawReaderMenuSection| {
            unreadable_items.extend(raw.unreadable.into_iter().map(|(key, error)| {
                ReaderUnreadableMenuItem {
                    section,
                    key,
                    error,
                }
            }));
            raw.items
        };
        Self {
            default_items: items(ReaderMenuSection::Default, raw.default),
            subscribed_items: items(ReaderMenuSection::Subscribed, raw.subscribed),
            recommended_items: items(ReaderMenuSection::Recommended, raw.recommended),
            unreadable_items,
        }
    }
}

/// A section's entries in the order the API sends them, and the key and error of each one that
/// couldn't be read.
#[derive(Default)]
struct RawReaderMenuSection {
    items: Vec<ReaderMenuItem>,
    unreadable: Vec<(String, String)>,
}

impl RawReaderMenuSection {
    fn push(&mut self, key: String, entry: serde_json::Value) {
        match serde_json::from_value::<RawReaderMenuItem>(entry) {
            Ok(raw) => self.items.push(ReaderMenuItem {
                key,
                kind: ReaderMenuItemKind::from_type(raw.kind.as_deref()),
                title: raw.title,
                url: raw.url,
                slug: raw.slug.as_deref().map(percent_decoded),
                display_name: raw.display_name,
                tag_id: raw.id,
                organization_id: raw.organization_id,
            }),
            Err(error) => self.unreadable.push((key, error.to_string())),
        }
    }
}

/// A section is an object of entries by name. With none, the API sends an empty list. Going
/// through `serde_json::Value` would sort the entries by name.
impl<'de> Deserialize<'de> for RawReaderMenuSection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SectionVisitor;

        impl<'de> Visitor<'de> for SectionVisitor {
            type Value = RawReaderMenuSection;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an object of menu entries by name")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut section = RawReaderMenuSection::default();
                while let Some((key, entry)) = map.next_entry::<String, serde_json::Value>()? {
                    section.push(key, entry);
                }
                Ok(section)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut section = RawReaderMenuSection::default();
                let mut position = 0;
                while let Some(entry) = seq.next_element::<serde_json::Value>()? {
                    section.push(position.to_string(), entry);
                    position += 1;
                }
                Ok(section)
            }

            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(RawReaderMenuSection::default())
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(RawReaderMenuSection::default())
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(RawReaderMenuSection::default())
            }
        }

        deserializer.deserialize_any(SectionVisitor)
    }
}

/// Every field a menu entry can arrive with. A followed tag's `ID` is a string, and a recommended
/// tag's is a number.
#[derive(Deserialize)]
struct RawReaderMenuItem {
    #[serde(
        rename = "type",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    kind: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    title: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    slug: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    display_name: Option<String>,
    #[serde(
        rename = "ID",
        default,
        deserialize_with = "deserialize_u64_or_string_or_none_as_t"
    )]
    id: Option<ReaderTagId>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    organization_id: Option<u64>,
}

/// Response from `GET /rest/v1.2/read/tags/{slug}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderTagResponse {
    pub tag: ReaderTag,
}

/// A tag.
///
/// It doesn't say whether the user follows the tag: the API sends the same to every reader. The
/// tags the user follows are in [`ReaderMenu::subscribed_items`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderTag {
    #[serde(
        rename = "ID",
        default,
        deserialize_with = "deserialize_u64_or_string_or_none_as_t"
    )]
    #[uniffi(default = None)]
    pub id: Option<ReaderTagId>,
    /// In lower case, whatever case the tag was asked for in.
    #[serde(deserialize_with = "deserialize_tag_slug")]
    pub slug: ReaderTagSlug,
    /// The name to show. HTML: it can carry character references, such as `Art &#038; Design`.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub title: Option<String>,
    /// The tag's name in lower case.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub display_name: Option<String>,
    /// `None` for every tag tried.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub description: Option<String>,
    /// The API address of the tag's posts.
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub url: Option<String>,
}

/// Response from `GET /wpcom/v2/read/interests`: the tags a new reader picks from, in the order
/// to show them. There are 50, in the request's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderInterestsResponse {
    pub interests: Vec<ReaderInterest>,
}

/// A tag a new reader can pick. Its title and slug are both in the request's language: `Food` and
/// `food` in English are `Alimentación` and `alimentacion` in Spanish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderInterest {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub title: Option<String>,
    #[serde(deserialize_with = "deserialize_tag_slug")]
    pub slug: ReaderTagSlug,
}

/// A tag's slug, percent-decoded.
fn deserialize_tag_slug<'de, D>(deserializer: D) -> Result<ReaderTagSlug, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(|slug| ReaderTagSlug(percent_decoded(&slug)))
}

/// Text with its `%XX` sequences decoded. Left as it is when what they decode to isn't UTF-8.
pub(crate) fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escaped = bytes
            .get(index + 1..index + 3)
            .filter(|hex| bytes[index] == b'%' && hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).unwrap_or_else(|_| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    fn keys(items: &[ReaderMenuItem]) -> Vec<&str> {
        items.iter().map(|item| item.key.as_str()).collect()
    }

    /// A signed-in menu recorded from WordPress.com, trimmed, with the user's tags and list
    /// replaced. It keeps the types the API sent: a followed tag's `ID` is a string, and a
    /// recommended tag's is a number.
    #[test]
    fn test_deserialize_menu() {
        let json = include_str!("../../tests/wpcom/reader_tags/menu.json");
        let menu: ReaderMenu = serde_json::from_str(json).expect("the menu parses");

        assert_eq!(
            keys(&menu.default_items),
            [
                "following",
                "discover",
                "liked",
                "a8c",
                "p2",
                "list-example-list"
            ]
        );
        assert!(menu.unreadable_items.is_empty());

        let following = &menu.default_items[0];
        assert_eq!(following.kind, ReaderMenuItemKind::Stream);
        assert_eq!(following.title.as_deref(), Some("Followed Sites"));
        assert_eq!(
            following.url.as_deref(),
            Some("https://public-api.wordpress.com/rest/v1.2/read/following")
        );
        assert_eq!(following.slug, None);

        let a8c = &menu.default_items[3];
        assert_eq!(a8c.kind, ReaderMenuItemKind::Organization);
        assert_eq!(a8c.slug.as_deref(), Some("a8c"));
        assert_eq!(a8c.organization_id, Some(1));
        assert_eq!(menu.default_items[4].organization_id, Some(2));

        let list = &menu.default_items[5];
        assert_eq!(list.kind, ReaderMenuItemKind::List);
        assert_eq!(list.title.as_deref(), Some("Example List"));
        assert_eq!(list.slug, None);

        // In the order the API sends them, which isn't the order of their keys.
        assert_eq!(keys(&menu.subscribed_items), ["9002", "9001"]);
        let followed = &menu.subscribed_items[0];
        assert_eq!(followed.kind, ReaderMenuItemKind::Tag);
        assert_eq!(followed.tag_id, Some(ReaderTagId(9002)));
        assert_eq!(followed.slug.as_deref(), Some("example-tag"));
        assert_eq!(followed.display_name.as_deref(), Some("example-tag"));

        assert_eq!(keys(&menu.recommended_items), ["2806", "178", "436"]);
        let recommended = &menu.recommended_items[0];
        assert_eq!(recommended.kind, ReaderMenuItemKind::Tag);
        assert_eq!(recommended.tag_id, Some(ReaderTagId(2806)));
        assert_eq!(recommended.title.as_deref(), Some("Art &amp; Design"));
    }

    /// What the API sends without a token: Discover alone, no followed tags, and the recommended
    /// tags.
    #[test]
    fn test_deserialize_signed_out_menu() {
        let json = include_str!("../../tests/wpcom/reader_tags/menu_signed_out.json");
        let menu: ReaderMenu = serde_json::from_str(json).expect("the menu parses");

        assert_eq!(keys(&menu.default_items), ["discover"]);
        assert!(menu.subscribed_items.is_empty());
        assert_eq!(keys(&menu.recommended_items), ["2806", "178"]);
        assert!(menu.unreadable_items.is_empty());
    }

    #[rstest]
    #[case::empty_list(json!([]))]
    #[case::empty_object(json!({}))]
    #[case::null(json!(null))]
    #[case::false_(json!(false))]
    fn test_a_section_with_no_entries(#[case] section: serde_json::Value) {
        let menu: ReaderMenu =
            serde_json::from_value(json!({ "subscribed": section })).expect("the menu parses");
        assert!(menu.subscribed_items.is_empty());
        assert!(menu.default_items.is_empty(), "A missing section is empty");
        assert!(menu.unreadable_items.is_empty());
    }

    #[test]
    fn test_an_entry_of_an_unknown_kind_is_kept() {
        let menu: ReaderMenu = serde_json::from_value(json!({
            "default": {
                "discover": { "title": "Discover", "URL": "https://example.com/discover" },
                "new-thing": {
                    "title": "New Thing",
                    "URL": "https://example.com/new",
                    "type": "hologram",
                    "depth": { "of": "field" },
                },
            },
        }))
        .expect("the menu parses");

        assert_eq!(menu.default_items.len(), 2);
        let unknown = menu
            .default_items
            .iter()
            .find(|item| item.key == "new-thing")
            .expect("the entry is kept");
        assert_eq!(
            unknown.kind,
            ReaderMenuItemKind::Other("hologram".to_string())
        );
        assert_eq!(unknown.title.as_deref(), Some("New Thing"));
        assert!(menu.unreadable_items.is_empty());
    }

    #[test]
    fn test_an_entry_that_cannot_be_read_does_not_fail_the_menu() {
        let menu: ReaderMenu = serde_json::from_str(
            r#"{
                "subscribed": {
                    "1": { "ID": "1", "title": "One", "slug": "one", "type": "tag" },
                    "2": "not an entry",
                    "3": { "ID": { "not": "an ID" }, "slug": "three", "type": "tag" },
                    "4": { "ID": 4, "title": "Four", "slug": "four", "type": "tag" }
                }
            }"#,
        )
        .expect("the menu parses");

        assert_eq!(keys(&menu.subscribed_items), ["1", "4"]);
        let unreadable: Vec<_> = menu
            .unreadable_items
            .iter()
            .map(|item| (item.section, item.key.as_str()))
            .collect();
        assert_eq!(
            unreadable,
            [
                (ReaderMenuSection::Subscribed, "2"),
                (ReaderMenuSection::Subscribed, "3")
            ]
        );
        assert!(!menu.unreadable_items[0].error.is_empty());
    }

    /// PHP sends an object whose keys are in sequence as a list.
    #[test]
    fn test_a_section_sent_as_a_list() {
        let menu: ReaderMenu = serde_json::from_value(json!({
            "recommended": [{ "ID": 178, "title": "Books", "slug": "books", "type": "tag" }],
        }))
        .expect("the menu parses");

        assert_eq!(keys(&menu.recommended_items), ["0"]);
        assert_eq!(menu.recommended_items[0].tag_id, Some(ReaderTagId(178)));
    }

    #[rstest]
    #[case(json!({ "default": "streams" }))]
    #[case(json!({ "subscribed": 7 }))]
    fn test_a_section_that_is_not_one_is_an_error(#[case] menu: serde_json::Value) {
        assert!(serde_json::from_value::<ReaderMenu>(menu).is_err());
    }

    #[test]
    fn test_a_menu_tag_slug_is_decoded() {
        let menu: ReaderMenu = serde_json::from_value(json!({
            "subscribed": {
                "18073": {
                    "ID": "18073",
                    "title": "写真",
                    "slug": "%e5%86%99%e7%9c%9f",
                    "display_name": "写真",
                    "type": "tag",
                },
            },
        }))
        .expect("the menu parses");
        assert_eq!(menu.subscribed_items[0].slug.as_deref(), Some("写真"));
    }

    #[test]
    fn test_deserialize_tag() {
        let json = include_str!("../../tests/wpcom/reader_tags/tag.json");
        let response: ReaderTagResponse = serde_json::from_str(json).expect("the tag parses");

        assert_eq!(response.tag.id, Some(ReaderTagId(436)));
        assert_eq!(response.tag.slug, ReaderTagSlug("photography".to_string()));
        assert_eq!(response.tag.title.as_deref(), Some("Photography"));
        assert_eq!(response.tag.display_name.as_deref(), Some("photography"));
        assert_eq!(response.tag.description, None);
        assert_eq!(
            response.tag.url.as_deref(),
            Some("https://public-api.wordpress.com/rest/v1.2/read/tags/photography/posts")
        );
    }

    /// What the API sends for a tag whose slug isn't ASCII.
    #[test]
    fn test_a_tag_slug_is_decoded() {
        let response: ReaderTagResponse = serde_json::from_value(json!({
            "tag": {
                "ID": "18073",
                "slug": "%e5%86%99%e7%9c%9f",
                "title": "写真",
                "display_name": "写真",
                "description": null,
                "URL": "https://public-api.wordpress.com/rest/v1.2/read/tags/%e5%86%99%e7%9c%9f/posts",
            },
        }))
        .expect("the tag parses");
        assert_eq!(response.tag.slug, ReaderTagSlug("写真".to_string()));
    }

    #[test]
    fn test_a_tag_without_a_slug_is_an_error() {
        let result = serde_json::from_value::<ReaderTagResponse>(
            json!({ "tag": { "ID": "436", "title": "Photography" } }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_interests() {
        let json = include_str!("../../tests/wpcom/reader_tags/interests.json");
        let response: ReaderInterestsResponse =
            serde_json::from_str(json).expect("the interests parse");

        let interests: Vec<_> = response
            .interests
            .iter()
            .map(|interest| (interest.title.as_deref(), interest.slug.0.as_str()))
            .collect();
        assert_eq!(
            interests,
            [
                (Some("Food"), "food"),
                (Some("Social Media"), "social-media"),
                (Some("食べ物"), "食べ物")
            ]
        );
    }

    #[rstest]
    #[case("photography", "photography")]
    #[case("%e5%86%99%e7%9c%9f", "写真")]
    #[case("%E5%86%99%E7%9C%9F", "写真")]
    #[case("caf%c3%a9-life", "café-life")]
    #[case::not_an_escape("100%", "100%")]
    #[case::not_hex("%zz%+1", "%zz%+1")]
    #[case::not_utf8("%ff%fe", "%ff%fe")]
    #[case::empty("", "")]
    fn test_percent_decoded(#[case] text: &str, #[case] expected: &str) {
        assert_eq!(percent_decoded(text), expected);
    }
}
