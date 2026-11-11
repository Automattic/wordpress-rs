//! Reader posts, as the Reader streams return them.
//!
//! Every Reader stream (Following, a site's or a feed's posts, tags, search, Liked) returns the same
//! post shape, with fields that vary in type between posts: a site ID can be a number or a string,
//! an empty `site_icon` or `tags` is an empty array rather than an object, and `i_like` can be `0` or
//! `1`. [`ReaderPost`] reads all of them into one shape.

use crate::{
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    html_entities::decode_html_entities,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::{
        WpComSiteId,
        reader_saved::{ReaderPostSaveIdentity, deserialize_optional_text},
    },
};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::{
    deserialize_null_as_empty_vec, deserialize_optional_bool_or_int,
    deserialize_u64_or_string_or_none,
};

/// A post in a Reader stream.
///
/// [`Self::identity`] is always set, and is what saving, liking and marking as seen need. Everything
/// else can be missing: an RSS post has no likes, and a stream only sends some fields when asked
/// with `meta`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(try_from = "RawReaderPost")]
pub struct ReaderPost {
    pub identity: ReaderPostSaveIdentity,
    /// The same for a post wherever the Reader shows it.
    #[uniffi(default = None)]
    pub global_id: Option<String>,
    #[uniffi(default = None)]
    pub pseudo_id: Option<String>,
    #[uniffi(default = None)]
    pub short_url: Option<String>,
    #[uniffi(default = None)]
    pub title: Option<String>,
    /// HTML.
    #[uniffi(default = None)]
    pub excerpt: Option<String>,
    /// HTML, as the site sent it.
    #[uniffi(default = None)]
    pub content: Option<String>,
    /// Whether the site asks readers to see the excerpt rather than the whole post.
    #[uniffi(default = None)]
    pub use_excerpt: Option<bool>,
    #[uniffi(default = None)]
    pub date: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub modified: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub author: Option<ReaderPostAuthor>,
    #[uniffi(default = None)]
    pub site_name: Option<String>,
    #[uniffi(default = None)]
    pub site_url: Option<String>,
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    /// The site's icon: `site_icon.img`, else the icon in `meta=site`, else an RSS feed's image
    /// from `meta=feed`. `site_icon.ico`, often a small favicon, is the last resort. Feeds that
    /// publish no image have none.
    #[uniffi(default = None)]
    pub site_icon_url: Option<String>,
    /// The site's language, such as `en`. Only sent when the stream is asked for `meta=site`.
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
    #[uniffi(default = None)]
    pub featured_image: Option<String>,
    #[uniffi(default = None)]
    pub featured_media: Option<ReaderFeaturedMedia>,
    /// The image to show for the post, picked the way the Reader apps pick it. See
    /// [`reader_post_display_image_url`] for which wins.
    ///
    /// Parsing a response sets it. When you build a post yourself, such as one to pass to a save,
    /// set it to the image you show for the post. Left `None`, the cache falls back to
    /// [`Self::featured_image`], then [`Self::featured_media`] when it's an image.
    #[uniffi(default = None)]
    pub display_image_url: Option<String>,
    pub tags: Vec<ReaderPostTag>,
    /// Set when the post is a cross-post of another.
    #[uniffi(default = None)]
    pub cross_post: Option<ReaderCrossPost>,
    #[uniffi(default = None)]
    pub like_count: Option<u64>,
    #[uniffi(default = None)]
    pub i_like: Option<bool>,
    #[uniffi(default = None)]
    pub likes_enabled: Option<bool>,
    #[uniffi(default = None)]
    pub is_following: Option<bool>,
    /// `None` when the post doesn't support seen status.
    #[uniffi(default = None)]
    pub is_seen: Option<bool>,
    #[uniffi(default = None)]
    pub comment_count: Option<u64>,
    #[uniffi(default = None)]
    pub comments_open: Option<bool>,
    #[uniffi(default = None)]
    pub can_subscribe_comments: Option<bool>,
    #[uniffi(default = None)]
    pub is_subscribed_comments: Option<bool>,
    #[uniffi(default = None)]
    pub subscribed_comments_notifications: Option<bool>,
    #[uniffi(default = None)]
    pub word_count: Option<u64>,
    /// When the user liked the post. Only the Liked stream sends it.
    #[uniffi(default = None)]
    pub date_liked: Option<WpGmtDateTime>,
    /// When the post was tagged. Only tag streams send it.
    #[uniffi(default = None)]
    pub tagged_on: Option<WpGmtDateTime>,
    /// Where Discover found the post, as raw JSON.
    #[uniffi(default = None)]
    pub discover_metadata: Option<String>,
    /// The analytics context of where the post was shown, as raw JSON, to send back with events.
    #[uniffi(default = None)]
    pub railcar: Option<String>,
}

/// The author of a [`ReaderPost`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostAuthor {
    /// `0` for an RSS post's author.
    #[serde(rename = "ID", default, deserialize_with = "deserialize_optional_id")]
    #[uniffi(default = None)]
    pub id: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub first_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub nice_name: Option<String>,
    #[serde(
        rename = "avatar_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub avatar_url: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub url: Option<String>,
    /// The author's own site. `None` when there isn't one, which a Jetpack site sends as `-1`.
    #[serde(
        rename = "site_ID",
        default,
        deserialize_with = "deserialize_optional_id"
    )]
    #[uniffi(default = None)]
    pub site_id: Option<u64>,
}

/// A post's featured image or video.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderFeaturedMedia {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub uri: Option<String>,
    /// `image` or `video`.
    #[serde(
        rename = "type",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    #[uniffi(default = None)]
    pub media_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    #[uniffi(default = None)]
    pub width: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    #[uniffi(default = None)]
    pub height: Option<u64>,
}

/// The image to show for a post, as the Reader apps pick it:
///
/// 1. `editorial.image`, which an editor picked for a post in Discover, unless it's a `wp.com/mshots/`
///    screenshot of the page, which the apps never show.
/// 2. `featured_image`, the image the author set.
/// 3. `featured_media.uri`, when `featured_media` is an image rather than a video. The server fills
///    it in from the post's images when the author set none.
///
/// Empty values are skipped. The saved list doesn't send `editorial`.
pub fn reader_post_display_image_url(
    editorial_image: Option<&str>,
    featured_image: Option<&str>,
    featured_media: Option<&ReaderFeaturedMedia>,
) -> Option<String> {
    let featured_media_image = featured_media
        .filter(|media| media.media_type.as_deref() == Some("image"))
        .and_then(|media| media.uri.as_deref());
    editorial_image
        .filter(|url| !url.contains("wp.com/mshots/"))
        .into_iter()
        .chain(featured_image)
        .chain(featured_media_image)
        .find(|url| !url.is_empty())
        .map(str::to_string)
}

/// A tag on a [`ReaderPost`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostTag {
    pub slug: String,
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub display_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    #[uniffi(default = None)]
    pub post_count: Option<u64>,
}

/// The post a cross-post copies, from the post's `metadata`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderCrossPost {
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    #[uniffi(default = None)]
    pub post_id: Option<u64>,
    #[uniffi(default = None)]
    pub post_url: Option<String>,
    /// Set when the cross-post is of a comment.
    #[uniffi(default = None)]
    pub comment_url: Option<String>,
}

/// The dates a page of a Reader stream covers, to page with `before`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderDateRange {
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
}

/// A page of a Reader stream, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderPostsResponse {
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    pub posts: Vec<ReaderPost>,
    /// The dates the page covers. The Following stream and feed streams send it; site streams don't.
    #[serde(default)]
    #[uniffi(default = None)]
    pub date_range: Option<ReaderDateRange>,
    /// How many posts there are. Site streams send it.
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    #[uniffi(default = None)]
    pub found: Option<u64>,
}

/// Query parameters for the Reader streams that page by date: Following, a site's posts, and a
/// feed's posts. For the next page, pass the oldest post's date as `before`.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The streams page by date rather than page number.
pub struct ReaderStreamParams {
    /// The number of posts to return. The apps ask for 7.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Only return posts before this time.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts after this time.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// Extra data to include, comma-separated: `site` sends each post's site, which has its
    /// [`ReaderPost::site_locale`], and `likes` its likes.
    #[uniffi(default = None)]
    pub meta: Option<String>,
}

/// Every field a Reader post can arrive with, before the ones that vary in shape are tidied up.
#[derive(Deserialize)]
struct RawReaderPost {
    #[serde(flatten)]
    identity: ReaderPostSaveIdentity,
    #[serde(
        rename = "global_ID",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    global_id: Option<String>,
    #[serde(
        rename = "pseudo_ID",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    pseudo_id: Option<String>,
    #[serde(
        rename = "short_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    short_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    excerpt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    use_excerpt: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    date: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    modified: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    author: Option<ReaderPostAuthor>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    site_name: Option<String>,
    #[serde(
        rename = "site_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    site_url: Option<String>,
    #[serde(
        rename = "feed_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    feed_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    site_icon: Option<RawSiteIcon>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    meta: Option<RawMeta>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    site_is_private: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    site_is_atomic: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_jetpack: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    organization_id: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    featured_image: Option<String>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    featured_media: Option<ReaderFeaturedMedia>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    editorial: Option<RawEditorial>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    tags: Option<BTreeMap<String, ReaderPostTag>>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    metadata: Option<Vec<RawMetadata>>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    like_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    i_like: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    likes_enabled: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_following: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_seen: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    discussion: Option<RawDiscussion>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    can_subscribe_comments: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_subscribed_comments: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    subscribed_comments_notifications: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    word_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    date_liked: Option<WpGmtDateTime>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    tagged_on: Option<WpGmtDateTime>,
    #[serde(default)]
    discover_metadata: Option<serde_json::Value>,
    #[serde(default)]
    railcar: Option<serde_json::Value>,
}

/// `editorial`, which Discover sends for a post an editor picked. Only its image is kept.
#[derive(Deserialize)]
struct RawEditorial {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    image: Option<String>,
}

#[derive(Deserialize)]
struct RawSiteIcon {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    img: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    ico: Option<String>,
}

/// `meta`, of which only the site's locale and icon and the feed's image are kept. `meta=site` adds
/// the whole site here, and `meta=feed` the whole feed.
#[derive(Deserialize)]
struct RawMeta {
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    data: Option<RawMetaData>,
}

#[derive(Deserialize)]
struct RawMetaData {
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    site: Option<RawMetaSite>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    feed: Option<RawMetaFeed>,
}

#[derive(Deserialize)]
struct RawMetaSite {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    locale: Option<String>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    icon: Option<RawSiteIcon>,
}

#[derive(Deserialize)]
struct RawMetaFeed {
    /// HTML-escaped: `&` arrives as `&amp;`.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    image: Option<String>,
}

#[derive(Deserialize)]
struct RawMetadata {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawDiscussion {
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    comment_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    comments_open: Option<bool>,
}

impl TryFrom<RawReaderPost> for ReaderPost {
    type Error = String;

    fn try_from(raw: RawReaderPost) -> Result<Self, Self::Error> {
        let metadata = raw.metadata.unwrap_or_default();
        let discussion = raw.discussion;
        let meta = raw.meta.and_then(|meta| meta.data);
        let meta_site = meta.as_ref().and_then(|data| data.site.as_ref());
        let meta_site_icon = meta_site.and_then(|site| site.icon.as_ref());
        let feed_image = meta
            .as_ref()
            .and_then(|data| data.feed.as_ref())
            .and_then(|feed| feed.image.clone())
            .map(decode_html_entities);
        let site_icon_url = raw
            .site_icon
            .as_ref()
            .and_then(|icon| icon.img.clone())
            .or_else(|| meta_site_icon.and_then(|icon| icon.img.clone()))
            .or(feed_image)
            .or_else(|| raw.site_icon.as_ref().and_then(|icon| icon.ico.clone()))
            .or_else(|| meta_site_icon.and_then(|icon| icon.ico.clone()));
        let site_locale = meta_site.and_then(|site| site.locale.clone());
        let display_image_url = reader_post_display_image_url(
            raw.editorial
                .as_ref()
                .and_then(|editorial| editorial.image.as_deref()),
            raw.featured_image.as_deref(),
            raw.featured_media.as_ref(),
        );
        Ok(Self {
            identity: raw.identity,
            global_id: raw.global_id,
            pseudo_id: raw.pseudo_id,
            short_url: raw.short_url,
            title: raw.title,
            excerpt: raw.excerpt,
            content: raw.content,
            use_excerpt: raw.use_excerpt,
            date: raw.date,
            modified: raw.modified,
            author: raw.author,
            site_name: raw.site_name,
            site_url: raw.site_url,
            feed_url: raw.feed_url,
            site_icon_url,
            site_locale,
            site_is_private: raw.site_is_private,
            site_is_atomic: raw.site_is_atomic,
            is_jetpack: raw.is_jetpack,
            organization_id: raw.organization_id,
            featured_image: raw.featured_image,
            featured_media: raw.featured_media,
            display_image_url,
            tags: raw.tags.unwrap_or_default().into_values().collect(),
            cross_post: cross_post(&metadata),
            like_count: raw.like_count,
            i_like: raw.i_like,
            likes_enabled: raw.likes_enabled,
            is_following: raw.is_following,
            is_seen: raw.is_seen,
            comment_count: discussion.as_ref().and_then(|d| d.comment_count),
            comments_open: discussion.as_ref().and_then(|d| d.comments_open),
            can_subscribe_comments: raw.can_subscribe_comments,
            is_subscribed_comments: raw.is_subscribed_comments,
            subscribed_comments_notifications: raw.subscribed_comments_notifications,
            word_count: raw.word_count,
            date_liked: raw.date_liked,
            tagged_on: raw.tagged_on,
            discover_metadata: raw
                .discover_metadata
                .filter(|value| !value.is_null())
                .map(|value| value.to_string()),
            railcar: raw
                .railcar
                .filter(|value| !value.is_null())
                .map(|value| value.to_string()),
        })
    }
}

/// The cross-post details in a post's `metadata`: `xpost_origin` is `"{site_id}:{post_id}"`, and
/// the permalinks are their own keys.
fn cross_post(metadata: &[RawMetadata]) -> Option<ReaderCrossPost> {
    let value = |key: &str| {
        metadata
            .iter()
            .find(|entry| entry.key.as_deref() == Some(key))
            .and_then(|entry| entry.value.as_ref())
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let origin = value("xpost_origin");
    let post_url = value("_xpost_original_permalink");
    let comment_url = value("xcomment_original_permalink");
    if origin.is_none() && post_url.is_none() && comment_url.is_none() {
        return None;
    }
    let (site_id, post_id) = origin
        .as_deref()
        .and_then(|origin| origin.split_once(':'))
        .map(|(site, post)| (site.parse().ok().map(WpComSiteId), post.parse().ok()))
        .unwrap_or((None, None));
    Some(ReaderCrossPost {
        site_id,
        post_id,
        post_url,
        comment_url,
    })
}

/// An ID the API can send as a number or a string, and as `-1` or `false` when there isn't one.
fn deserialize_optional_id<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::Number(number)) => number.as_u64(),
            Some(serde_json::Value::String(text)) => text.parse().ok(),
            _ => None,
        },
    )
}

/// A value the API sends as an object, or as an empty object or array, an empty string, `false` or
/// `null` when there isn't one.
fn deserialize_object_or_empty<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) | Some(serde_json::Value::Bool(false)) => Ok(None),
        Some(serde_json::Value::Array(items)) if items.is_empty() => Ok(None),
        Some(serde_json::Value::Object(fields)) if fields.is_empty() => Ok(None),
        Some(serde_json::Value::String(text)) if text.is_empty() => Ok(None),
        Some(value) => serde_json::from_value(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::posts::PostId;
    use crate::wp_com::reader_saved::{
        ReaderFeedItemId, ReaderSavedKey, reader_saved_key_for_post,
    };
    use serde_json::json;

    fn parse(value: serde_json::Value) -> ReaderPost {
        serde_json::from_value(value).expect("the post parses")
    }

    /// A recording of a site's posts on WordPress.com, with `meta=site,likes`, trimmed.
    #[test]
    fn test_deserialize_site_posts() {
        let json = include_str!("../../tests/wpcom/reader_posts/site_posts.json");
        let response: ReaderPostsResponse =
            serde_json::from_str(json).expect("Failed to deserialize site posts");
        assert_eq!(response.found, Some(2));
        assert_eq!(response.date_range, None);
        let post = &response.posts[0];
        assert_eq!(
            reader_saved_key_for_post(post.identity.clone()),
            Some(ReaderSavedKey::Post {
                blog_id: WpComSiteId(3584907),
                post_id: PostId(post.identity.id as i64),
            })
        );
        assert!(post.title.is_some() && post.content.is_some());
        assert_eq!(post.site_name.as_deref(), Some("WordPress.com Blog"));
        assert_eq!(post.site_locale.as_deref(), Some("en"));
        assert!(post.site_icon_url.is_some());
        assert!(!post.tags.is_empty());
        assert!(post.author.as_ref().and_then(|a| a.name.as_ref()).is_some());
        assert!(post.comment_count.is_some());
        assert_eq!(post.site_is_private, Some(false));
    }

    /// A recording of an RSS feed's posts on WordPress.com, trimmed.
    #[test]
    fn test_deserialize_feed_posts() {
        let json = include_str!("../../tests/wpcom/reader_posts/feed_posts.json");
        let response: ReaderPostsResponse =
            serde_json::from_str(json).expect("Failed to deserialize feed posts");
        assert!(response.date_range.and_then(|range| range.before).is_some());
        let post = &response.posts[0];
        assert!(post.identity.is_external);
        assert!(matches!(
            reader_saved_key_for_post(post.identity.clone()),
            Some(ReaderSavedKey::FeedItem { .. })
        ));
        assert_eq!(post.site_name.as_deref(), Some("xkcd.com"));
        assert_eq!(post.site_icon_url, None);
        assert!(post.tags.is_empty());
        assert_eq!(post.site_locale, None, "Not asked for with meta=site");
    }

    #[test]
    fn test_fields_that_vary_in_shape() {
        let post = parse(json!({
            "ID": 9,
            "site_ID": "123",
            "is_external": false,
            "site_icon": [],
            "tags": [],
            "i_like": 1,
            "metadata": false,
            "author": { "ID": "7", "name": "", "email": false, "site_ID": -1 },
            "featured_media": {},
            "discussion": { "comment_count": "4", "comments_open": true },
            "date": "",
        }));
        assert_eq!(post.identity.site_id, Some(WpComSiteId(123)));
        assert_eq!(post.site_icon_url, None);
        assert!(post.tags.is_empty());
        assert_eq!(post.i_like, Some(true));
        assert_eq!(post.cross_post, None);
        assert_eq!(post.author.as_ref().unwrap().id, Some(7));
        assert_eq!(post.author.as_ref().unwrap().name, None);
        assert_eq!(
            post.author.as_ref().unwrap().site_id,
            None,
            "A Jetpack site sends -1"
        );
        assert_eq!(post.featured_media, None);
        assert_eq!(post.comment_count, Some(4));
        assert_eq!(post.date, None);
    }

    #[rstest::rstest]
    #[case::own_icon(
        json!({ "img": "https://a/img.png", "ico": "https://a/favicon.ico" }),
        json!({ "site": { "icon": { "img": "https://site/img.png" } }, "feed": { "image": "https://feed/img.png" } }),
        Some("https://a/img.png")
    )]
    #[case::site_meta_icon(
        json!({ "ico": "https://a/favicon.ico" }),
        json!({ "site": { "icon": { "img": "https://site/img.png" } }, "feed": { "image": "https://feed/img.png" } }),
        Some("https://site/img.png")
    )]
    #[case::feed_image_unescaped(
        json!([]),
        json!({ "feed": { "image": "https://feed/img.png?w=150&amp;h=150" } }),
        Some("https://feed/img.png?w=150&h=150")
    )]
    #[case::favicon_last(
        json!({ "ico": "https://a/favicon.ico" }),
        json!({ "feed": { "image": "" } }),
        Some("https://a/favicon.ico")
    )]
    #[case::site_meta_favicon_last(
        json!([]),
        json!({ "site": { "icon": { "ico": "https://site/favicon.ico" } }, "feed": {} }),
        Some("https://site/favicon.ico")
    )]
    #[case::none(json!([]), json!([]), None)]
    fn test_site_icon_url(
        #[case] site_icon: serde_json::Value,
        #[case] meta_data: serde_json::Value,
        #[case] expected: Option<&str>,
    ) {
        let post = parse(json!({
            "ID": 1,
            "site_ID": 2,
            "is_external": false,
            "site_icon": site_icon,
            "meta": { "data": meta_data },
        }));
        assert_eq!(post.site_icon_url.as_deref(), expected);
    }

    const MSHOTS: &str = "https://s0.wp.com/mshots/v1/https%3A%2F%2Fexample.com%2Fpost?w=252";

    #[rstest::rstest]
    #[case::editorial_first(
        json!({ "image": "https://editorial/img.jpg" }),
        json!("https://featured/img.jpg"),
        json!({ "uri": "https://media/img.jpg", "type": "image" }),
        Some("https://editorial/img.jpg")
    )]
    #[case::editorial_mshots_skipped(
        json!({ "image": MSHOTS }),
        json!("https://featured/img.jpg"),
        json!({ "uri": "https://media/img.jpg", "type": "image" }),
        Some("https://featured/img.jpg")
    )]
    #[case::featured_image(
        json!({ "image": "" }),
        json!("https://featured/img.jpg"),
        json!({ "uri": "https://media/img.jpg", "type": "image" }),
        Some("https://featured/img.jpg")
    )]
    #[case::featured_media_image(
        json!(null),
        json!(""),
        json!({ "uri": "https://media/img.jpg", "type": "image" }),
        Some("https://media/img.jpg")
    )]
    #[case::featured_media_not_an_image(
        json!(null),
        json!(""),
        json!({ "uri": "https://media/video.mp4", "type": "video" }),
        None
    )]
    #[case::editorial_mshots_and_nothing_else(json!({ "image": MSHOTS }), json!(""), json!([]), None)]
    #[case::none(json!(null), json!(null), json!(null), None)]
    fn test_display_image_url(
        #[case] editorial: serde_json::Value,
        #[case] featured_image: serde_json::Value,
        #[case] featured_media: serde_json::Value,
        #[case] expected: Option<&str>,
    ) {
        let post = parse(json!({
            "ID": 1,
            "site_ID": 2,
            "is_external": false,
            "editorial": editorial,
            "featured_image": featured_image,
            "featured_media": featured_media,
        }));
        assert_eq!(post.display_image_url.as_deref(), expected);
    }

    #[test]
    fn test_an_rss_post_without_likes_or_seen_status() {
        let post = parse(json!({
            "ID": 55,
            "site_ID": 5,
            "feed_ID": 5,
            "feed_item_ID": 55,
            "is_external": true,
        }));
        assert_eq!(post.identity.feed_item_id, Some(ReaderFeedItemId(55)));
        assert_eq!(post.like_count, None);
        assert_eq!(
            post.is_seen, None,
            "Absent means the post doesn't support it"
        );
    }

    #[test]
    fn test_cross_post() {
        let post = parse(json!({
            "ID": 1,
            "site_ID": 2,
            "is_external": false,
            "metadata": [
                { "id": "1", "key": "xpost_origin", "value": "123:456" },
                { "id": "2", "key": "_xpost_original_permalink", "value": "https://example.com/p" },
                { "id": "3", "key": "other", "value": "x" },
            ],
        }));
        assert_eq!(
            post.cross_post,
            Some(ReaderCrossPost {
                site_id: Some(WpComSiteId(123)),
                post_id: Some(456),
                post_url: Some("https://example.com/p".to_string()),
                comment_url: None,
            })
        );
    }

    #[test]
    fn test_stream_specific_fields() {
        let post = parse(json!({
            "ID": 1,
            "site_ID": 2,
            "is_external": false,
            "date_liked": "2026-11-01T10:00:00+00:00",
            "tagged_on": "2026-11-02T10:00:00+00:00",
            "railcar": { "railcar": "abc", "fetch_algo": "x" },
            "discover_metadata": null,
        }));
        assert!(post.date_liked.is_some() && post.tagged_on.is_some());
        let railcar: serde_json::Value = serde_json::from_str(&post.railcar.unwrap()).unwrap();
        assert_eq!(railcar["railcar"], "abc");
        assert_eq!(post.discover_metadata, None);
    }

    #[test]
    fn test_a_post_without_an_id_is_an_error() {
        assert!(serde_json::from_value::<ReaderPost>(json!({ "site_ID": 2 })).is_err());
    }
}
