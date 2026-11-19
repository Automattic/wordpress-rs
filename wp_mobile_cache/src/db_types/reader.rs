//! Types for the Reader tables.

use crate::{RowId, SqliteDbError, list_metadata::ListState};
use std::str::FromStr;
use wp_api::date::WpGmtDateTime;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_post_display::{
    ReaderPostCardType, reader_post_card_type, reader_post_reading_time_minutes,
    reader_post_summary,
};
use wp_api::wp_com::reader_posts::{
    ReaderCrossPost, ReaderPost, ReaderPostTag, reader_post_display_image_url,
};
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
    #[uniffi(default = None)]
    pub short_url: Option<String>,
    /// Whether the site asks readers to see the excerpt rather than the whole post.
    #[uniffi(default = None)]
    pub use_excerpt: Option<bool>,
    /// `0` for an RSS post's author.
    #[uniffi(default = None)]
    pub author_id: Option<u64>,
    /// The site's language, such as `en`.
    #[uniffi(default = None)]
    pub site_locale: Option<String>,
    #[uniffi(default = None)]
    pub site_is_private: Option<bool>,
    #[uniffi(default = None)]
    pub site_is_atomic: Option<bool>,
    #[uniffi(default = None)]
    pub is_jetpack: Option<bool>,
    /// Set for an Automattic (`1`) or P2 (`2`) site.
    #[uniffi(default = None)]
    pub organization_id: Option<u64>,
    /// Set when the post is a cross-post of another.
    #[uniffi(default = None)]
    pub cross_post: Option<ReaderCrossPost>,
    #[uniffi(default = None)]
    pub likes_enabled: Option<bool>,
    #[uniffi(default = None)]
    pub comments_open: Option<bool>,
    #[uniffi(default = None)]
    pub can_subscribe_comments: Option<bool>,
    #[uniffi(default = None)]
    pub is_subscribed_comments: Option<bool>,
    /// Whether the user gets notifications for the post's comments.
    #[uniffi(default = None)]
    pub receives_comment_notifications: Option<bool>,
    /// `None` when the post doesn't support seen status.
    #[uniffi(default = None)]
    pub is_seen: Option<bool>,
    #[uniffi(default = None)]
    pub word_count: Option<u64>,
    /// The text to show on the post's card, worked out when the post was stored. See
    /// [`reader_post_summary`]. `None` for a post with no text, and for one stored from a source
    /// that sent neither its excerpt nor its content.
    #[uniffi(default = None)]
    pub summary: Option<String>,
    /// How long the post takes to read, in minutes, worked out when the post was stored. See
    /// [`reader_post_reading_time_minutes`]. `None` when the source sent no words to count.
    #[uniffi(default = None)]
    pub reading_time_minutes: Option<u32>,
    /// Which layout the post gets in a stream, worked out when the post was stored. See
    /// [`reader_post_card_type`]. `None` when the source didn't send the post's content, which
    /// the layout depends on.
    #[uniffi(default = None)]
    pub card_type: Option<ReaderPostCardType>,
}

/// A card type as the cache stores it. The numbers never change, since rows outlive a launch.
pub fn reader_card_type_number(card_type: ReaderPostCardType) -> i64 {
    match card_type {
        ReaderPostCardType::Default => 0,
        ReaderPostCardType::Photo => 1,
        ReaderPostCardType::Gallery => 2,
        ReaderPostCardType::Video => 3,
        ReaderPostCardType::CrossPost => 4,
    }
}

/// Reads a card type stored by [`reader_card_type_number`]. A number this version doesn't know,
/// which a newer version stored, reads as the default layout.
pub fn reader_card_type_from_number(number: i64) -> ReaderPostCardType {
    match number {
        1 => ReaderPostCardType::Photo,
        2 => ReaderPostCardType::Gallery,
        3 => ReaderPostCardType::Video,
        4 => ReaderPostCardType::CrossPost,
        _ => ReaderPostCardType::Default,
    }
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
            short_url: None,
            use_excerpt: None,
            author_id: None,
            site_locale: None,
            site_is_private: None,
            site_is_atomic: None,
            is_jetpack: None,
            organization_id: None,
            cross_post: None,
            likes_enabled: None,
            comments_open: None,
            can_subscribe_comments: None,
            is_subscribed_comments: None,
            receives_comment_notifications: None,
            is_seen: None,
            word_count: None,
            summary: None,
            reading_time_minutes: None,
            card_type: None,
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
            short_url: post.short_url.clone(),
            use_excerpt: post.use_excerpt,
            author_id: author.and_then(|author| author.id),
            site_locale: post.site_locale.clone(),
            site_is_private: post.site_is_private,
            site_is_atomic: post.site_is_atomic,
            is_jetpack: post.is_jetpack,
            organization_id: post.organization_id,
            cross_post: post.cross_post.clone(),
            likes_enabled: post.likes_enabled,
            comments_open: post.comments_open,
            can_subscribe_comments: post.can_subscribe_comments,
            is_subscribed_comments: post.is_subscribed_comments,
            receives_comment_notifications: post.subscribed_comments_notifications,
            is_seen: post.is_seen,
            word_count: post.word_count,
            summary: reader_post_summary(post),
            reading_time_minutes: reader_post_reading_time_minutes(post),
            // The layout is read from the content, so a source that sent none can't say.
            card_type: post.content.as_ref().map(|_| reader_post_card_type(post)),
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
    /// Posts may be missing between this one and the one above it.
    pub gap_before: bool,
}

/// A post on a page of a stream, to add to the stream's items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderPageItem {
    /// The post's row in `reader_posts`.
    pub post: RowId,
    /// Where the post sorts in a stream ordered by date: [`reader_date_text`] of the date the
    /// stream orders by. Not used by a stream kept in the order the server sent.
    pub sort_key: String,
    pub railcar: Option<String>,
}

/// Something on a page of a stream kept in the server's order: a post, or a card that isn't one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbReaderPageEntry {
    Post(DbReaderPageItem),
    /// A card that isn't a post, as JSON.
    Card(String),
}

/// A card in a stream that isn't a post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderStreamCard {
    /// The card, as JSON.
    pub card: String,
    /// The key of the post the card comes after, or `None` if it comes before every post.
    pub after_post: Option<ReaderSavedKey>,
}

/// What adding a page to a stream did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DbReaderPageOutcome {
    /// Posts the stream didn't have.
    pub added: u64,
    /// Posts the stream had that the server no longer returns.
    pub removed: u64,
    /// Whether the stream already had one of the page's posts.
    pub overlapped: bool,
}

/// A post a stream's page had that couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderUnreadablePost {
    pub row_id: RowId,
    pub stream_row_id: RowId,
    pub identity: ReaderPostSaveIdentity,
    pub error: String,
    /// How many times it's been asked for on its own since.
    pub attempts: u32,
    /// When a page first had it.
    pub first_seen_at: WpGmtDateTime,
    pub last_attempt_at: Option<WpGmtDateTime>,
}

/// One save on the server, as far as this device knows, and what the user wants for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderSavedRow {
    pub row_id: RowId,
    /// The key the server stores, once an answer or a refresh says. Until then, the key sent.
    pub key: ReaderSavedKey,
    /// The URL keys the server stores as [`Self::key`]. A post can have several.
    pub url_keys: Vec<ReaderSavedKey>,
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
