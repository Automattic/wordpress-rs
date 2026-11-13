//! Types for the Reader tables.

use crate::{RowId, SqliteDbError, list_metadata::ListState};
use std::str::FromStr;
use wp_api::date::WpGmtDateTime;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_posts::{ReaderPost, ReaderPostTag, reader_post_display_image_url};
use wp_api::wp_com::reader_saved::{
    ReaderFeedItemId, ReaderPostSaveIdentity, ReaderSavedKey, reader_saved_key_for_post,
};

/// A Reader post as the cache stores it.
///
/// Only some sources send the whole post, so everything but the identity can be missing. The saved
/// list, for example, may send little more than the identity.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderPostRecord {
    /// The key the post is stored under: the key it would be saved under, normalized.
    pub key: ReaderSavedKey,
    pub identity: ReaderPostSaveIdentity,
    #[uniffi(default = None)]
    pub global_id: Option<String>,
    #[uniffi(default = None)]
    pub title: Option<String>,
    #[uniffi(default = None)]
    pub excerpt: Option<String>,
    /// Raw HTML, as the API sent it.
    #[uniffi(default = None)]
    pub content: Option<String>,
    #[uniffi(default = None)]
    pub date: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub modified: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub author_name: Option<String>,
    #[uniffi(default = None)]
    pub author_avatar_url: Option<String>,
    #[uniffi(default = None)]
    pub author_url: Option<String>,
    /// The featured image the author set, as the server sends it. Most posts without one still
    /// have an image to show: use [`Self::display_image_url`] for that.
    #[uniffi(default = None)]
    pub featured_image: Option<String>,
    /// The image to show for the post: [`Self::editorial_image_url`] when there is one, else
    /// [`Self::featured_image`], else the image the server picked from the post. See
    /// [`ReaderPost::display_image_url`].
    #[uniffi(default = None)]
    pub display_image_url: Option<String>,
    /// The image an editor picked for the post in Discover. Once stored, a source that doesn't send
    /// one leaves it, and [`Self::display_image_url`] is this image. See
    /// [`ReaderPost::editorial_image_url`].
    #[uniffi(default = None)]
    pub editorial_image_url: Option<String>,
    #[uniffi(default = None)]
    pub site_name: Option<String>,
    #[uniffi(default = None)]
    pub site_url: Option<String>,
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    #[uniffi(default = None)]
    pub like_count: Option<u64>,
    #[uniffi(default = None)]
    pub i_like: Option<bool>,
    #[uniffi(default = None)]
    pub is_following: Option<bool>,
    /// The site's icon, or an RSS feed's image. See [`ReaderPost::site_icon_url`] for which wins.
    #[uniffi(default = None)]
    pub site_icon_url: Option<String>,
    /// `None` when the source didn't send tags, and empty when the post has none.
    #[uniffi(default = None)]
    pub tags: Option<Vec<ReaderPostTag>>,
    /// The server sends `0` for a post that can't have comments, such as an RSS post or a page
    /// saved by URL.
    #[uniffi(default = None)]
    pub comment_count: Option<u64>,
}

impl ReaderPostRecord {
    /// A record with only the identity set, or `None` for a post that has no key.
    pub fn from_identity(identity: ReaderPostSaveIdentity) -> Option<Self> {
        let key = reader_saved_key_for_post(identity.clone())?.normalized();
        Some(Self {
            key,
            identity,
            global_id: None,
            title: None,
            excerpt: None,
            content: None,
            date: None,
            modified: None,
            author_name: None,
            author_avatar_url: None,
            author_url: None,
            featured_image: None,
            display_image_url: None,
            editorial_image_url: None,
            site_name: None,
            site_url: None,
            feed_url: None,
            like_count: None,
            i_like: None,
            is_following: None,
            site_icon_url: None,
            tags: None,
            comment_count: None,
        })
    }

    /// The fields a Reader stream sent, or `None` for a post that has no key.
    pub fn from_reader_post(post: &ReaderPost) -> Option<Self> {
        let author = post.author.as_ref();
        Some(Self {
            global_id: post.global_id.clone(),
            title: post.title.clone(),
            excerpt: post.excerpt.clone(),
            content: post.content.clone(),
            date: post.date,
            modified: post.modified,
            author_name: author.and_then(|author| author.name.clone()),
            author_avatar_url: author.and_then(|author| author.avatar_url.clone()),
            author_url: author.and_then(|author| author.url.clone()),
            featured_image: post.featured_image.clone(),
            // Stored without the editor's pick, which has its own column, so a source that sends no
            // pick still updates the image shown when there isn't one.
            display_image_url: post
                .display_image_url
                .clone()
                .filter(|url| Some(url) != post.editorial_image_url.as_ref())
                .or_else(|| {
                    reader_post_display_image_url(
                        None,
                        post.featured_image.as_deref(),
                        post.featured_media.as_ref(),
                    )
                }),
            editorial_image_url: post.editorial_image_url.clone(),
            site_name: post.site_name.clone(),
            site_url: post.site_url.clone(),
            feed_url: post.feed_url.clone(),
            like_count: post.like_count,
            i_like: post.i_like,
            is_following: post.is_following,
            site_icon_url: post.site_icon_url.clone(),
            tags: post.tags.clone(),
            comment_count: post.comment_count,
            ..Self::from_identity(post.identity.clone())?
        })
    }
}

/// A Reader stream's sync state and where it's up to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderStream {
    pub row_id: RowId,
    pub next_page_handle: Option<String>,
    /// `None` until the first page loads.
    pub has_more: Option<bool>,
    pub last_refreshed_at: Option<WpGmtDateTime>,
    pub state: ListState,
    pub error_message: Option<String>,
    pub version: i64,
}

/// A post in a stream, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderStreamItem {
    pub post: ReaderPostRecord,
    pub post_row_id: RowId,
    pub sort_key: String,
}

/// One save on the server, as far as this device knows, and what the user wants for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderSavedRow {
    pub row_id: RowId,
    /// The key the server stores, once an answer or a refresh says. Until then, the key sent.
    pub key: ReaderSavedKey,
    /// A URL key the server stores as [`Self::key`].
    pub url_key: Option<ReaderSavedKey>,
    /// The feed item key of a WordPress.com post saved under its IDs.
    pub feed_item_key: Option<ReaderSavedKey>,
    /// What the user wants.
    pub saved: bool,
    /// What the server last said, or `None` if it hasn't.
    pub server: Option<bool>,
    /// Sent without an answer, so it isn't sent again until a refresh says what the server has.
    pub in_doubt: bool,
    /// A tap's request isn't answered yet, so it's sent even if [`Self::saved`] matches
    /// [`Self::server`]: the server may have changed since it last said.
    pub tapped: bool,
    /// The order of the last tap on this device, or 0 for none.
    pub seq: i64,
    pub date_saved: Option<WpGmtDateTime>,
    /// Whether [`Self::date_saved`] came from the server. A tap's date doesn't: saving a post
    /// already saved keeps its old date there.
    pub date_known: bool,
    pub post_row_id: Option<RowId>,
    /// Shown in the saved list: the list sent it, or this device saved it.
    pub listed: bool,
    /// From an import, so it can be sent in a bulk request.
    pub imported: bool,
    /// The server refused the save for the account's save cap, with no tap waiting to hear it. It
    /// stays saved, and is sent again once there's room.
    pub waiting_for_room: bool,
}

impl DbReaderSavedRow {
    /// Whether the server doesn't have what the user wants yet, including a save waiting for room.
    pub fn is_pending(&self) -> bool {
        self.server != Some(self.saved)
    }
}

/// What the saved posts service knows about an account besides its rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DbReaderSavedState {
    /// Whether a refresh of the saved keys has been applied.
    pub keys_refreshed: bool,
    /// Whether the newest refresh returned every saved key: past 1,000 it doesn't.
    pub keys_complete: bool,
    /// The newest total the server gave.
    pub total: Option<u64>,
}

/// One entry of a refresh of the saved keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderSavedKey {
    pub key: ReaderSavedKey,
    pub date_saved: WpGmtDateTime,
    /// The URL of a post saved by its IDs, when the server sent it.
    pub url: Option<String>,
}

/// A row of the saved list, with its post when the cache has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderSavedListItem {
    pub row: DbReaderSavedRow,
    pub post: Option<ReaderPostRecord>,
}

/// A key as stored text: `post:{blog_id}:{post_id}`, `feed-item:{feed_item_id}` or `url:{url}`,
/// normalized first. The same form the iOS app uses for its local saves.
pub fn reader_saved_key_text(key: &ReaderSavedKey) -> String {
    match key.normalized() {
        ReaderSavedKey::Post { blog_id, post_id } => format!("post:{}:{}", blog_id.0, post_id.0),
        ReaderSavedKey::FeedItem { feed_item_id } => format!("feed-item:{}", feed_item_id.0),
        ReaderSavedKey::Url { url } => format!("url:{url}"),
    }
}

/// Reads a key written by [`reader_saved_key_text`].
pub fn reader_saved_key_from_text(text: &str) -> Result<ReaderSavedKey, SqliteDbError> {
    let invalid = || SqliteDbError::SqliteError(format!("Invalid Reader saved key: {text}"));
    if let Some(url) = text.strip_prefix("url:") {
        return Ok(ReaderSavedKey::Url {
            url: url.to_string(),
        });
    }
    if let Some(id) = text.strip_prefix("feed-item:") {
        let id = id.parse().map_err(|_| invalid())?;
        return Ok(ReaderSavedKey::FeedItem {
            feed_item_id: ReaderFeedItemId(id),
        });
    }
    let (blog_id, post_id) = text
        .strip_prefix("post:")
        .and_then(|ids| ids.split_once(':'))
        .ok_or_else(invalid)?;
    Ok(ReaderSavedKey::Post {
        blog_id: WpComSiteId(blog_id.parse().map_err(|_| invalid())?),
        post_id: PostId(post_id.parse().map_err(|_| invalid())?),
    })
}

/// A date as stored text that sorts in date order: UTC, with milliseconds.
pub fn reader_date_text(date: &WpGmtDateTime) -> String {
    date.0.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Reads a date written by [`reader_date_text`], or by SQLite's `strftime`.
pub fn reader_date_from_text(text: &str) -> Result<WpGmtDateTime, SqliteDbError> {
    WpGmtDateTime::from_str(text)
        .map_err(|e| SqliteDbError::SqliteError(format!("Invalid date {text:?}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(ReaderSavedKey::Post { blog_id: WpComSiteId(123), post_id: PostId(456) }, "post:123:456")]
    #[case(ReaderSavedKey::FeedItem { feed_item_id: ReaderFeedItemId(789) }, "feed-item:789")]
    #[case(ReaderSavedKey::Url { url: "https://example.com/a:b".to_string() }, "url:https://example.com/a:b")]
    fn test_key_text_round_trips(#[case] key: ReaderSavedKey, #[case] text: &str) {
        assert_eq!(reader_saved_key_text(&key), text);
        assert_eq!(reader_saved_key_from_text(text).unwrap(), key);
    }

    #[test]
    fn test_key_text_is_normalized() {
        let key = ReaderSavedKey::Url {
            url: "https://example.com/page/?utm_source=x".to_string(),
        };
        assert_eq!(reader_saved_key_text(&key), "url:https://example.com/page");
    }

    #[rstest]
    #[case("post:1")]
    #[case("post:a:b")]
    #[case("feed-item:")]
    #[case("other:1")]
    fn test_invalid_key_text_is_an_error(#[case] text: &str) {
        assert!(reader_saved_key_from_text(text).is_err());
    }

    fn reader_post(fields: serde_json::Value) -> ReaderPost {
        let mut post = serde_json::json!({ "ID": 1, "site_ID": 2, "is_external": false });
        post.as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        serde_json::from_value(post).unwrap()
    }

    #[test]
    fn test_record_keeps_a_parsed_posts_display_image() {
        let post = reader_post(serde_json::json!({
            "featured_image": "",
            "featured_media": { "uri": "https://example.com/media.jpg", "type": "image" },
        }));
        let record = ReaderPostRecord::from_reader_post(&post).unwrap();
        assert_eq!(record.featured_image, None);
        assert_eq!(
            record.display_image_url.as_deref(),
            Some("https://example.com/media.jpg")
        );
    }

    #[test]
    fn test_record_keeps_an_editorial_image_apart() {
        let post = reader_post(serde_json::json!({
            "editorial": { "image": "https://example.com/editorial.jpg" },
            "featured_image": "https://example.com/featured.jpg",
        }));
        assert_eq!(
            post.display_image_url.as_deref(),
            Some("https://example.com/editorial.jpg")
        );
        let record = ReaderPostRecord::from_reader_post(&post).unwrap();
        assert_eq!(
            record.editorial_image_url.as_deref(),
            Some("https://example.com/editorial.jpg")
        );
        assert_eq!(
            record.display_image_url.as_deref(),
            Some("https://example.com/featured.jpg"),
            "Stored without the pick, which has its own column"
        );
    }

    /// A post an app builds itself may set only `featured_image` or `featured_media`.
    #[rstest]
    #[case::own_display_image(
        Some("https://app/display.jpg"),
        Some("https://app/featured.jpg"),
        Some("https://app/display.jpg")
    )]
    #[case::featured_image(
        None,
        Some("https://app/featured.jpg"),
        Some("https://app/featured.jpg")
    )]
    #[case::featured_media(None, None, Some("https://app/media.jpg"))]
    fn test_record_falls_back_for_a_post_built_by_an_app(
        #[case] display_image_url: Option<&str>,
        #[case] featured_image: Option<&str>,
        #[case] expected: Option<&str>,
    ) {
        let mut post = reader_post(serde_json::json!({}));
        post.display_image_url = display_image_url.map(str::to_string);
        post.featured_image = featured_image.map(str::to_string);
        post.featured_media = Some(wp_api::wp_com::reader_posts::ReaderFeaturedMedia {
            uri: Some("https://app/media.jpg".to_string()),
            media_type: Some("image".to_string()),
            width: None,
            height: None,
        });
        let record = ReaderPostRecord::from_reader_post(&post).unwrap();
        assert_eq!(record.display_image_url.as_deref(), expected);
    }

    #[test]
    fn test_date_text_round_trips_and_sorts() {
        let earlier: WpGmtDateTime = "2026-01-02T03:04:05Z".parse().unwrap();
        let later: WpGmtDateTime = "2026-01-02T03:04:05.5Z".parse().unwrap();
        assert_eq!(reader_date_text(&earlier), "2026-01-02T03:04:05.000Z");
        assert_eq!(
            reader_date_from_text(&reader_date_text(&later)).unwrap(),
            later
        );
        assert!(reader_date_text(&earlier) < reader_date_text(&later));
    }
}
