use crate::{
    date::WpGmtDateTime,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::WpComSiteId,
    wp_content_u64_id,
};
use serde::{Deserialize, Serialize};
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::{
    deserialize_false_as_none, deserialize_false_or_string_or_null, deserialize_null_as_empty_vec,
    deserialize_optional_bool_or_int, deserialize_u64_or_string, deserialize_u64_or_string_or_none,
    deserialize_u64_or_string_or_none_as_t,
};

wp_content_u64_id!(ReaderFeedId);
wp_content_u64_id!(ReaderFeedItemId);
wp_content_u64_id!(ReaderSaveId);

/// Identifies a saved post.
///
/// The API stores a feed item that belongs to a WordPress.com post under its `blog_id` + `post_id`.
/// A Jetpack post can be saved under either, so check both when showing whether a post is saved;
/// [`reader_saved_keys_for_post`] returns them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged, try_from = "RawReaderSavedKey")]
pub enum ReaderSavedKey {
    /// A WordPress.com post.
    Post {
        blog_id: WpComSiteId,
        post_id: PostId,
    },
    /// An RSS post.
    FeedItem { feed_item_id: ReaderFeedItemId },
    /// A page that isn't a Reader post. Saving a URL can still return one of the other keys: a
    /// URL already saved keeps its key, and the URL of a WordPress.com post resolves to the post.
    Url { url: String },
}

/// Identifies a post in a bulk import, which can't save a URL.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged, try_from = "RawReaderSavedKey")]
pub enum ReaderSavedBulkKey {
    /// A WordPress.com post.
    Post {
        blog_id: WpComSiteId,
        post_id: PostId,
    },
    /// An RSS post.
    FeedItem { feed_item_id: ReaderFeedItemId },
}

/// Every field a key can arrive with. Any kind of save can carry a `url`, so it only identifies a
/// save that has no IDs, and `0` means an ID isn't set.
#[derive(Default, Deserialize)]
struct RawReaderSavedKey {
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    blog_id: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    post_id: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    feed_item_id: Option<u64>,
    #[serde(default)]
    url: Option<String>,
}

impl TryFrom<RawReaderSavedKey> for ReaderSavedKey {
    type Error = String;

    fn try_from(raw: RawReaderSavedKey) -> Result<Self, Self::Error> {
        let feed_item_id = raw.feed_item_id.filter(|id| *id != 0);
        let blog_id = raw.blog_id.filter(|id| *id != 0);
        let post_id = raw.post_id.filter(|id| *id != 0);
        match (feed_item_id, blog_id, post_id, raw.url) {
            (Some(feed_item_id), ..) => Ok(Self::FeedItem {
                feed_item_id: ReaderFeedItemId(feed_item_id),
            }),
            (None, Some(blog_id), Some(post_id), _) => Ok(Self::Post {
                blog_id: WpComSiteId(blog_id),
                post_id: PostId(
                    i64::try_from(post_id)
                        .map_err(|_| format!("`post_id` {post_id} is out of range"))?,
                ),
            }),
            (None, _, _, Some(url)) if !url.is_empty() => Ok(Self::Url { url }),
            _ => Err("a saved key needs `feed_item_id`, `blog_id` + `post_id`, or `url`".into()),
        }
    }
}

impl TryFrom<RawReaderSavedKey> for ReaderSavedBulkKey {
    type Error = String;

    fn try_from(raw: RawReaderSavedKey) -> Result<Self, Self::Error> {
        match ReaderSavedKey::try_from(raw)? {
            ReaderSavedKey::Post { blog_id, post_id } => Ok(Self::Post { blog_id, post_id }),
            ReaderSavedKey::FeedItem { feed_item_id } => Ok(Self::FeedItem { feed_item_id }),
            ReaderSavedKey::Url { .. } => Err("a bulk key can't be a `url`".into()),
        }
    }
}

/// One entry of `GET /wpcom/v2/read/saved/keys`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(try_from = "RawReaderSavedKeyEntry")]
pub struct ReaderSavedKeyEntry {
    #[serde(flatten)]
    pub key: ReaderSavedKey,
    pub date_saved: WpGmtDateTime,
    /// The save's own ID.
    pub id: ReaderSaveId,
    /// The URL of a post saved by its IDs, when the save has one. A page saved by URL has it in
    /// [`Self::key`] instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Deserialize)]
struct RawReaderSavedKeyEntry {
    #[serde(flatten)]
    key: RawReaderSavedKey,
    date_saved: WpGmtDateTime,
    id: ReaderSaveId,
}

impl TryFrom<RawReaderSavedKeyEntry> for ReaderSavedKeyEntry {
    type Error = String;

    fn try_from(raw: RawReaderSavedKeyEntry) -> Result<Self, Self::Error> {
        let url = raw.key.url.clone();
        let key = ReaderSavedKey::try_from(raw.key)?;
        Ok(Self {
            url: url.filter(|_| !matches!(key, ReaderSavedKey::Url { .. })),
            key,
            date_saved: raw.date_saved,
            id: raw.id,
        })
    }
}

/// Response from `GET /wpcom/v2/read/saved/keys`: every saved key, newest first, up to 1,000.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSavedKeysResponse {
    pub keys: Vec<ReaderSavedKeyEntry>,
    pub total: u64,
}

/// The app making a save, recorded with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, uniffi::Enum)]
#[serde(rename_all = "lowercase")]
pub enum ReaderSavedSource {
    Android,
    Ios,
}

/// Request body for `POST /wpcom/v2/read/saved/new`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderSavePostParams {
    #[serde(flatten)]
    pub key: ReaderSavedKey,
    pub source: ReaderSavedSource,
    /// What to show for a page saved by URL, which the server doesn't fetch. Kept only when the URL
    /// isn't a WordPress.com post.
    #[serde(flatten)]
    #[uniffi(default = None)]
    pub page_details: Option<ReaderSavedPageDetails>,
}

/// The details of a page saved by URL, as plain text.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderSavedPageDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub site_name: Option<String>,
}

/// Request body for `POST /wpcom/v2/read/saved/delete`.
///
/// Send a key that `new` or `keys` returned. A `feed_item_id` for a WordPress.com post stops
/// resolving once the feed item is pruned, and then gets a `post_not_found` error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnsavePostParams {
    #[serde(flatten)]
    pub key: ReaderSavedKey,
    pub source: ReaderSavedSource,
}

/// Response from `POST /wpcom/v2/read/saved/new` and `POST /wpcom/v2/read/saved/delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSavedPostResponse {
    pub saved: bool,
    /// The key the API stored, which can differ from the one sent.
    pub key: ReaderSavedKey,
    /// How many posts are saved after this request.
    pub total: u64,
}

/// Request body for `POST /wpcom/v2/read/saved/bulk`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderSavedBulkParams {
    /// The keys to save, newest first: between 1 and 100, or the whole request is rejected. If they
    /// would take the user past the 1,000 save cap, the newest are the ones saved.
    pub keys: Vec<ReaderSavedBulkKey>,
    pub source: ReaderSavedSource,
}

/// Response from `POST /wpcom/v2/read/saved/bulk`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSavedBulkResponse {
    /// One result per key, in the order sent.
    pub results: Vec<ReaderSavedBulkResult>,
    /// How many posts are saved after this request.
    pub total: u64,
}

/// What happened to one key of a bulk import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(try_from = "RawReaderSavedBulkResult")]
pub struct ReaderSavedBulkResult {
    /// The key the API stored, which can differ from the one sent. `None` when the key was rejected
    /// and what the API sent back isn't a whole key: match it by its position in the results.
    pub key: Option<ReaderSavedBulkKey>,
    /// Whether the post is saved, including when it already was.
    pub saved: bool,
    /// Why the key wasn't saved, such as `invalid_input`, `post_not_found` or, once the account
    /// reaches its 1,000 save cap, `saved_limit_reached`.
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct RawReaderSavedBulkResult {
    #[serde(default)]
    key: Option<RawReaderSavedKey>,
    saved: bool,
    #[serde(default)]
    error: Option<String>,
}

impl TryFrom<RawReaderSavedBulkResult> for ReaderSavedBulkResult {
    type Error = String;

    fn try_from(raw: RawReaderSavedBulkResult) -> Result<Self, Self::Error> {
        // A rejected key comes back without its zero or malformed IDs, so `{ blog_id: 0, post_id: 5 }`
        // returns as `{ post_id: 5 }`. That can't fail the other 99 results.
        let key = match (raw.key.unwrap_or_default().try_into(), &raw.error) {
            (Ok(key), _) => Some(key),
            (Err(_), Some(_)) => None,
            (Err(error), None) => return Err(error),
        };
        Ok(Self {
            key,
            saved: raw.saved,
            error: raw.error,
        })
    }
}

/// Query parameters for `GET /rest/v1.2/read/saved`.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The list pages with `page_handle` rather than a page number.
pub struct ReaderSavedPostsParams {
    /// The previous page's [`ReaderSavedPostsResponse::next_page_handle`].
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
    /// The number of posts to return: 10 by default, and at most 40.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Only return posts saved after this time.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// Only return posts saved before this time. Ignored when [`Self::page_handle`] is set.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
}

/// Response from `GET /rest/v1.2/read/saved`, newest save first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSavedPostsResponse {
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub posts: Vec<ReaderSavedPost>,
    /// The handle of the next page. Only a missing handle means the end of the list: posts that
    /// can't be shown are skipped, so a page can be short, or even empty, with more to come.
    #[serde(default)]
    pub next_page_handle: Option<String>,
    /// How many posts the user has saved.
    #[serde(default)]
    pub found: Option<u64>,
}

/// The fields of a Reader post that decide which key it's saved under.
///
/// IDs stay as the API sent them, because a `0` means something different from a missing ID: see
/// [`Self::is_snapshot`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostSaveIdentity {
    /// The post's ID. On an RSS post it's the feed item ID, and on a page saved by URL it's the
    /// save's own ID.
    #[serde(rename = "ID", deserialize_with = "deserialize_u64_or_string")]
    pub id: u64,
    #[serde(
        rename = "site_ID",
        default,
        deserialize_with = "deserialize_u64_or_string_or_none_as_t"
    )]
    pub site_id: Option<WpComSiteId>,
    #[serde(
        rename = "feed_ID",
        default,
        deserialize_with = "deserialize_u64_or_string_or_none_as_t"
    )]
    pub feed_id: Option<ReaderFeedId>,
    #[serde(
        rename = "feed_item_ID",
        default,
        deserialize_with = "deserialize_u64_or_string_or_none_as_t"
    )]
    pub feed_item_id: Option<ReaderFeedItemId>,
    #[serde(default)]
    pub is_external: bool,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    pub url: Option<String>,
}

impl ReaderPostSaveIdentity {
    /// Whether this is a saved snapshot rather than a Reader post: a page saved by URL, or a feed
    /// item that's since been pruned.
    fn is_snapshot(&self) -> bool {
        self.is_external
            && self.site_id == Some(WpComSiteId(0))
            && self.feed_id == Some(ReaderFeedId(0))
    }

    fn nonzero_feed_item_id(&self) -> Option<ReaderFeedItemId> {
        self.feed_item_id.filter(|id| id.0 != 0)
    }
}

/// The key to save a post under: `feed_item_id` for an RSS post, `blog_id` + `post_id` for
/// everything else, or `None` when the post is missing the IDs it needs.
///
/// Matches `savedKeyForPost()` in wp-calypso, so a post is saved the same way on every platform.
#[uniffi::export]
pub fn reader_saved_key_for_post(post: ReaderPostSaveIdentity) -> Option<ReaderSavedKey> {
    if post.is_snapshot() && post.nonzero_feed_item_id().is_none() {
        return post
            .url
            .filter(|url| !url.is_empty())
            .map(|url| ReaderSavedKey::Url { url });
    }

    if post.is_external {
        let feed_item_id = post
            .nonzero_feed_item_id()
            .or(Some(ReaderFeedItemId(post.id)))
            .filter(|id| id.0 != 0)?;
        return Some(ReaderSavedKey::FeedItem { feed_item_id });
    }

    let blog_id = post.site_id.filter(|id| id.0 != 0)?;
    let post_id = i64::try_from(post.id).ok().filter(|id| *id != 0)?;
    Some(ReaderSavedKey::Post {
        blog_id,
        post_id: PostId(post_id),
    })
}

/// Every key a post may already be saved under, starting with [`reader_saved_key_for_post`].
///
/// A Jetpack post comes back as external while its WordPress.com copy fails to load, so it can be
/// saved under its feed item and shown later with a blog and post ID. Show the post as saved if any
/// of these keys is, and unsave with the keys that matched.
#[uniffi::export]
pub fn reader_saved_keys_for_post(post: ReaderPostSaveIdentity) -> Vec<ReaderSavedKey> {
    let feed_item_id = post.nonzero_feed_item_id().filter(|_| !post.is_external);
    let Some(key) = reader_saved_key_for_post(post) else {
        return vec![];
    };

    match feed_item_id {
        Some(feed_item_id) => vec![key, ReaderSavedKey::FeedItem { feed_item_id }],
        None => vec![key],
    }
}

/// Text the API can send as `null`, `false` or an empty string when there isn't any. A page saved by
/// URL with no details has an empty title and site name, for example.
fn deserialize_optional_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(deserialize_false_or_string_or_null(deserializer)?.filter(|text| !text.is_empty()))
}

/// A post in the saved list.
///
/// Only [`Self::identity`] is reliably set. A saved snapshot has no Reader post behind it, so
/// everything else can be missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSavedPost {
    #[serde(flatten)]
    pub identity: ReaderPostSaveIdentity,
    #[serde(
        default,
        deserialize_with = "crate::date::deserialize_optional_wp_gmt_date_time"
    )]
    pub date_saved: Option<WpGmtDateTime>,
    #[serde(
        rename = "global_ID",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    pub global_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    pub excerpt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    pub content: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::date::deserialize_optional_wp_gmt_date_time"
    )]
    pub date: Option<WpGmtDateTime>,
    #[serde(
        default,
        deserialize_with = "crate::date::deserialize_optional_wp_gmt_date_time"
    )]
    pub modified: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_false_as_none")]
    pub author: Option<ReaderSavedPostAuthor>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    pub featured_image: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    pub site_name: Option<String>,
    #[serde(
        rename = "site_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    pub site_url: Option<String>,
    #[serde(
        rename = "feed_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    pub feed_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    pub like_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    pub i_like: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    pub is_following: Option<bool>,
}

/// The author of a [`ReaderSavedPost`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSavedPostAuthor {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    pub name: Option<String>,
    #[serde(
        rename = "avatar_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    pub avatar_url: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    fn post_key() -> ReaderSavedKey {
        ReaderSavedKey::Post {
            blog_id: WpComSiteId(123),
            post_id: PostId(456),
        }
    }

    fn feed_item_key(id: u64) -> ReaderSavedKey {
        ReaderSavedKey::FeedItem {
            feed_item_id: ReaderFeedItemId(id),
        }
    }

    fn bulk_post_key() -> ReaderSavedBulkKey {
        ReaderSavedBulkKey::Post {
            blog_id: WpComSiteId(123),
            post_id: PostId(456),
        }
    }

    fn bulk_feed_item_key() -> ReaderSavedBulkKey {
        ReaderSavedBulkKey::FeedItem {
            feed_item_id: ReaderFeedItemId(789),
        }
    }

    fn url_key() -> ReaderSavedKey {
        ReaderSavedKey::Url {
            url: "https://example.com/page".to_string(),
        }
    }

    fn identity() -> ReaderPostSaveIdentity {
        ReaderPostSaveIdentity {
            id: 456,
            site_id: Some(WpComSiteId(123)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        }
    }

    fn deleted_feed_item_or_url_save() -> ReaderPostSaveIdentity {
        ReaderPostSaveIdentity {
            id: 9,
            site_id: Some(WpComSiteId(0)),
            feed_id: Some(ReaderFeedId(0)),
            is_external: true,
            url: Some("https://example.com/page".to_string()),
            ..identity()
        }
    }

    #[test]
    fn test_deserialize_keys_response() {
        let json = include_str!("../../tests/wpcom/reader_saved/keys.json");
        let response: ReaderSavedKeysResponse =
            serde_json::from_str(json).expect("Failed to deserialize keys response");
        assert_eq!(response.total, 3);

        let keys: Vec<_> = response
            .keys
            .iter()
            .map(|entry| entry.key.clone())
            .collect();
        assert_eq!(keys, vec![post_key(), feed_item_key(789), url_key()]);

        // A post saved with a URL is still identified by its IDs, and keeps the URL.
        assert_eq!(response.keys[0].id, ReaderSaveId(11));
        assert_eq!(
            response.keys[0].url.as_deref(),
            Some("https://example.com/2026/11/02/a-post")
        );
        assert_eq!(response.keys[1].url, None);
        assert_eq!(response.keys[2].url, None);
    }

    #[test]
    fn test_keys_response_round_trips() {
        let json = include_str!("../../tests/wpcom/reader_saved/keys.json");
        let response: ReaderSavedKeysResponse = serde_json::from_str(json).unwrap();
        let reparsed: ReaderSavedKeysResponse =
            serde_json::from_value(serde_json::to_value(&response).unwrap()).unwrap();
        assert_eq!(reparsed, response);
    }

    #[test]
    fn test_deserialize_list_response() {
        let json = include_str!("../../tests/wpcom/reader_saved/list.json");
        let response: ReaderSavedPostsResponse =
            serde_json::from_str(json).expect("Failed to deserialize list response");
        assert_eq!(response.next_page_handle.as_deref(), Some("handle=="));
        assert_eq!(response.found, Some(12));
        assert_eq!(response.posts.len(), 4);

        let [post, rss_post, pruned_feed_item, url_save] = &response.posts[..] else {
            panic!("Expected four posts");
        };

        assert_eq!(post.identity.id, 456);
        assert_eq!(post.title.as_deref(), Some("A WordPress.com post"));
        assert!(post.date_saved.is_some());
        assert_eq!(post.i_like, Some(false));
        assert_eq!(post.like_count, Some(3));
        assert_eq!(
            post.author.as_ref().and_then(|a| a.name.as_deref()),
            Some("An Author")
        );
        assert_eq!(
            reader_saved_key_for_post(post.identity.clone()),
            Some(post_key())
        );

        // Feed posts send `i_like` and `is_following` as integers.
        assert_eq!(rss_post.i_like, Some(false));
        assert_eq!(rss_post.is_following, Some(true));
        assert_eq!(
            reader_saved_key_for_post(rss_post.identity.clone()),
            Some(feed_item_key(789))
        );

        assert_eq!(pruned_feed_item.date, None);
        assert_eq!(
            reader_saved_key_for_post(pruned_feed_item.identity.clone()),
            Some(feed_item_key(790))
        );

        assert_eq!(
            reader_saved_key_for_post(url_save.identity.clone()),
            Some(url_key())
        );
    }

    #[test]
    fn test_deserialize_treats_empty_text_as_missing() {
        let post: ReaderSavedPost = serde_json::from_value(json!({
            "ID": 13,
            "site_ID": 0,
            "feed_ID": 0,
            "is_external": true,
            "URL": "https://example.com/page",
            "title": "",
            "site_name": "",
            "site_URL": "",
            "excerpt": "",
            "featured_image": "",
            "author": { "name": "" },
        }))
        .unwrap();

        assert_eq!(post.title, None);
        assert_eq!(post.site_name, None);
        assert_eq!(post.site_url, None);
        assert_eq!(post.excerpt, None);
        assert_eq!(post.featured_image, None);
        assert_eq!(post.author.and_then(|author| author.name), None);
        assert_eq!(
            post.identity.url.as_deref(),
            Some("https://example.com/page")
        );
    }

    #[test]
    fn test_deserialize_last_page_has_no_handle() {
        let response: ReaderSavedPostsResponse =
            serde_json::from_value(json!({ "posts": [] })).unwrap();
        assert_eq!(response.next_page_handle, None);
    }

    #[test]
    fn test_deserialize_mutation_response() {
        let json = include_str!("../../tests/wpcom/reader_saved/new.json");
        let response: ReaderSavedPostResponse =
            serde_json::from_str(json).expect("Failed to deserialize save response");
        assert!(response.saved);
        assert_eq!(response.key, post_key());
        assert_eq!(response.total, 4);
    }

    #[test]
    fn test_deserialize_bulk_response() {
        let json = include_str!("../../tests/wpcom/reader_saved/bulk.json");
        let response: ReaderSavedBulkResponse =
            serde_json::from_str(json).expect("Failed to deserialize bulk response");
        assert_eq!(response.total, 1000);
        assert_eq!(
            response.results,
            vec![
                ReaderSavedBulkResult {
                    key: Some(bulk_post_key()),
                    saved: true,
                    error: None,
                },
                ReaderSavedBulkResult {
                    key: Some(bulk_feed_item_key()),
                    saved: false,
                    error: Some("saved_limit_reached".to_string()),
                },
                // `{ blog_id: 0, post_id: 5 }`, without the zero.
                ReaderSavedBulkResult {
                    key: None,
                    saved: false,
                    error: Some("invalid_input".to_string()),
                },
                // `{ feed_item_id: 0 }`, likewise.
                ReaderSavedBulkResult {
                    key: None,
                    saved: false,
                    error: Some("invalid_input".to_string()),
                },
            ]
        );
    }

    #[test]
    fn test_deserialize_saved_bulk_result_needs_a_key() {
        let result = json!({ "key": { "post_id": 5 }, "saved": true });
        assert!(serde_json::from_value::<ReaderSavedBulkResult>(result).is_err());
    }

    #[rstest]
    #[case::post(post_key(), json!({ "blog_id": 123, "post_id": 456, "source": "android" }))]
    #[case::feed_item(feed_item_key(789), json!({ "feed_item_id": 789, "source": "android" }))]
    #[case::url(url_key(), json!({ "url": "https://example.com/page", "source": "android" }))]
    fn test_serialize_keys(#[case] key: ReaderSavedKey, #[case] expected: serde_json::Value) {
        let save = ReaderSavePostParams {
            key: key.clone(),
            source: ReaderSavedSource::Android,
            page_details: None,
        };
        assert_eq!(serde_json::to_value(save).unwrap(), expected);

        let unsave = ReaderUnsavePostParams {
            key,
            source: ReaderSavedSource::Android,
        };
        assert_eq!(serde_json::to_value(unsave).unwrap(), expected);
    }

    #[test]
    fn test_serialize_page_details() {
        let params = ReaderSavePostParams {
            key: url_key(),
            source: ReaderSavedSource::Ios,
            page_details: Some(ReaderSavedPageDetails {
                title: Some("A page".to_string()),
                image_url: Some("https://example.com/image.jpg".to_string()),
                ..Default::default()
            }),
        };
        assert_eq!(
            serde_json::to_value(params).unwrap(),
            json!({
                "url": "https://example.com/page",
                "source": "ios",
                "title": "A page",
                "image_url": "https://example.com/image.jpg",
            })
        );
    }

    #[test]
    fn test_serialize_bulk_params() {
        let params = ReaderSavedBulkParams {
            keys: vec![bulk_post_key(), bulk_feed_item_key()],
            source: ReaderSavedSource::Ios,
        };
        assert_eq!(
            serde_json::to_value(params).unwrap(),
            json!({
                "keys": [{ "blog_id": 123, "post_id": 456 }, { "feed_item_id": 789 }],
                "source": "ios",
            })
        );
    }

    #[rstest]
    #[case::string_ids(json!({ "blog_id": "123", "post_id": "456" }), post_key())]
    #[case::zero_feed_item_is_unset(
        json!({ "feed_item_id": 0, "blog_id": 123, "post_id": 456 }),
        post_key()
    )]
    #[case::zero_ids_are_unset(
        json!({ "blog_id": 0, "post_id": 0, "url": "https://example.com/page" }),
        url_key()
    )]
    // The same order as `getReadSavedKeyId()` in wp-calypso.
    #[case::feed_item_wins(
        json!({ "feed_item_id": 789, "blog_id": 123, "post_id": 456 }),
        feed_item_key(789)
    )]
    fn test_deserialize_key(#[case] value: serde_json::Value, #[case] expected: ReaderSavedKey) {
        assert_eq!(
            serde_json::from_value::<ReaderSavedKey>(value).unwrap(),
            expected
        );
    }

    #[rstest]
    #[case::empty(json!({}))]
    #[case::only_blog(json!({ "blog_id": 123 }))]
    #[case::empty_url(json!({ "url": "" }))]
    fn test_deserialize_key_without_an_identifier_fails(#[case] value: serde_json::Value) {
        assert!(serde_json::from_value::<ReaderSavedKey>(value).is_err());
    }

    #[rstest]
    #[case::wp_com_post(identity(), Some(post_key()))]
    #[case::missing_site(ReaderPostSaveIdentity { site_id: None, ..identity() }, None)]
    #[case::zero_site(
        ReaderPostSaveIdentity { site_id: Some(WpComSiteId(0)), ..identity() },
        None
    )]
    #[case::zero_post(ReaderPostSaveIdentity { id: 0, ..identity() }, None)]
    #[case::wp_com_post_read_through_following(
        ReaderPostSaveIdentity {
            feed_id: Some(ReaderFeedId(5)),
            feed_item_id: Some(ReaderFeedItemId(789)),
            ..identity()
        },
        Some(post_key())
    )]
    #[case::rss_post(
        ReaderPostSaveIdentity {
            is_external: true,
            feed_item_id: Some(ReaderFeedItemId(789)),
            ..identity()
        },
        Some(feed_item_key(789))
    )]
    // Without a `feed_item_ID`, an RSS post's `ID` is its feed item ID.
    #[case::rss_post_without_feed_item_id(
        ReaderPostSaveIdentity { id: 789, is_external: true, ..identity() },
        Some(feed_item_key(789))
    )]
    #[case::rss_post_without_ids(
        ReaderPostSaveIdentity { id: 0, is_external: true, ..identity() },
        None
    )]
    #[case::url_save(deleted_feed_item_or_url_save(), Some(url_key()))]
    #[case::url_save_without_url(
        ReaderPostSaveIdentity { url: None, ..deleted_feed_item_or_url_save() },
        None
    )]
    #[case::deleted_feed_item(
        ReaderPostSaveIdentity {
            feed_item_id: Some(ReaderFeedItemId(789)),
            ..deleted_feed_item_or_url_save()
        },
        Some(feed_item_key(789))
    )]
    fn test_saved_key_for_post(
        #[case] post: ReaderPostSaveIdentity,
        #[case] expected: Option<ReaderSavedKey>,
    ) {
        assert_eq!(reader_saved_key_for_post(post), expected);
    }

    #[rstest]
    #[case::wp_com_post(identity(), vec![post_key()])]
    #[case::jetpack_post_with_wp_com_copy(
        ReaderPostSaveIdentity { feed_item_id: Some(ReaderFeedItemId(789)), ..identity() },
        vec![post_key(), feed_item_key(789)]
    )]
    #[case::jetpack_post_returned_as_external(
        ReaderPostSaveIdentity {
            is_external: true,
            feed_item_id: Some(ReaderFeedItemId(789)),
            ..identity()
        },
        vec![feed_item_key(789)]
    )]
    #[case::unsaveable(ReaderPostSaveIdentity { id: 0, ..identity() }, vec![])]
    fn test_saved_keys_for_post(
        #[case] post: ReaderPostSaveIdentity,
        #[case] expected: Vec<ReaderSavedKey>,
    ) {
        assert_eq!(reader_saved_keys_for_post(post), expected);
    }
}
