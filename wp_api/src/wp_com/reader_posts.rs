//! Reader posts, as the Reader streams return them.
//!
//! Every Reader stream (Following, a site's or a feed's posts, tags, search, Liked) returns the same
//! post shape, with fields that vary in type between posts: a site ID can be a number or a string,
//! an empty `site_icon` or `tags` is an empty array rather than an object, and `i_like` can be `0` or
//! `1`. [`ReaderPost`] reads all of them into one shape.
//!
//! A post that still can't be read doesn't fail its page. The page keeps the posts that can be
//! read, and lists each of the others as a [`ReaderUnreadablePost`].

use crate::{
    date::{WpGmtDateTime, deserialize_optional_wp_gmt_date_time},
    html_entities::decode_html_entities,
    impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::{
        WpComSiteId,
        reader_saved::{ReaderFeedId, ReaderPostSaveIdentity, deserialize_optional_text},
    },
    wp_content_string_id,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeOwned, IgnoredAny, MapAccess, SeqAccess, Visitor},
};
use serde_json::value::RawValue;
use std::fmt;
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::{
    LenientVec, deserialize_optional_bool_or_int, deserialize_u64_or_string_or_none,
    deserialize_u64_or_string_or_none_as_t,
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
    /// HTML: it carries character references such as `&#8211;`. Decode it with
    /// `decode_html_entities` to show it as text.
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
    /// Can carry character references such as `&amp;`. Decode it with
    /// `decode_html_entities` to show it as text.
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
    /// The image an editor picked for the post in Discover, from `editorial.image`. `None` when the
    /// response has no `editorial`, which the saved list never sends, or when the pick is a
    /// `wp.com/mshots/` screenshot, which the apps never show.
    ///
    /// The cache keeps it once it has it, and shows it ahead of any other image, so a refresh from
    /// a source without `editorial` doesn't swap the post's image back to its featured image.
    #[uniffi(default = None)]
    pub editorial_image_url: Option<String>,
    /// When Freshly Pressed shows the post, from `editorial.displayed_on`. Freshly Pressed is
    /// ordered by it, newest first, and pages by it. The newest one can be a few hours in the
    /// future. Only Freshly Pressed sends it.
    #[uniffi(default = None)]
    pub editorial_displayed_on: Option<WpGmtDateTime>,
    /// In the order the API sends them. `None` when the response left out `tags`, and empty when
    /// the post has none.
    #[uniffi(default = None)]
    pub tags: Option<Vec<ReaderPostTag>>,
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
    /// Can carry character references such as `&amp;`. Decode it with
    /// `decode_html_entities` to show it as text.
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
    /// `None` when the API sends something that isn't a size, such as a negative number.
    #[serde(default, deserialize_with = "deserialize_optional_size")]
    #[uniffi(default = None)]
    pub width: Option<u64>,
    /// `None` when the API sends something that isn't a size, such as a negative number.
    #[serde(default, deserialize_with = "deserialize_optional_size")]
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
    reader_post_editorial_image_url(editorial_image)
        .into_iter()
        .chain(featured_image)
        .chain(featured_media_image)
        .find(|url| !url.is_empty())
        .map(str::to_string)
}

/// `editorial.image`, unless it's empty or a `wp.com/mshots/` screenshot of the page.
fn reader_post_editorial_image_url(editorial_image: Option<&str>) -> Option<&str> {
    editorial_image.filter(|url| !url.is_empty() && !url.contains("wp.com/mshots/"))
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
///
/// The API takes each date from a post on the page, so a date that can't be read turns up here as
/// well as on its post. That doesn't fail the page: the date is left out, and what the API sent is
/// kept beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderDateRange")]
pub struct ReaderDateRange {
    /// `None` when the API left it out, and when what it sent is in [`Self::unreadable_before`].
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// `None` when the API left it out, and when what it sent is in [`Self::unreadable_after`].
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// What the API sent for `before`, when that isn't a date that can be read.
    #[uniffi(default = None)]
    pub unreadable_before: Option<String>,
    /// What the API sent for `after`, when that isn't a date that can be read.
    #[uniffi(default = None)]
    pub unreadable_after: Option<String>,
}

#[derive(Deserialize)]
struct RawReaderDateRange {
    #[serde(default)]
    before: Option<serde_json::Value>,
    #[serde(default)]
    after: Option<serde_json::Value>,
}

impl From<RawReaderDateRange> for ReaderDateRange {
    fn from(raw: RawReaderDateRange) -> Self {
        let (before, unreadable_before) = date_range_bound(raw.before);
        let (after, unreadable_after) = date_range_bound(raw.after);
        Self {
            before,
            after,
            unreadable_before,
            unreadable_after,
        }
    }
}

/// One end of a date range: the date, or what the API sent when that isn't a date that can be read.
fn date_range_bound(sent: Option<serde_json::Value>) -> (Option<WpGmtDateTime>, Option<String>) {
    let Some(sent) = sent else {
        return (None, None);
    };
    match deserialize_optional_wp_gmt_date_time(&sent) {
        Ok(date) => (date, None),
        Err(_) => {
            let unreadable = match sent {
                serde_json::Value::String(text) => text,
                other => other.to_string(),
            };
            (None, Some(unreadable))
        }
    }
}

/// A post on a page of a Reader stream that couldn't be read.
///
/// One post the library can't read doesn't fail its page: the page keeps the posts it can read,
/// and lists the others as these, so that a caller can report them and knows the page had more
/// posts than it returned. WordPress.com sends a few such posts, for example one whose date has a
/// UTC offset of `+91:00`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnreadablePost {
    /// Which post it is. `None` when its IDs couldn't be read either.
    #[uniffi(default = None)]
    pub identity: Option<ReaderPostSaveIdentity>,
    /// Why it couldn't be read. A line and column in it count from the start of the post.
    pub error: String,
}

/// Sorts a page's posts into the ones that were read and the ones that weren't.
fn sort_posts<T>(posts: Option<LenientVec<T>>) -> (Vec<T>, Vec<ReaderUnreadablePost>) {
    let posts = posts.unwrap_or_default();
    let unreadable = posts
        .invalid
        .into_iter()
        .map(|post| unreadable_post(&post.json, post.error.to_string()))
        .collect();
    (posts.valid, unreadable)
}

/// A post that couldn't be read, with its IDs when those can be.
fn unreadable_post(json: &RawValue, error: String) -> ReaderUnreadablePost {
    ReaderUnreadablePost {
        identity: serde_json::from_str(json.get()).ok(),
        error,
    }
}

/// A page of a Reader stream that pages by date, newest first.
///
/// For the next page, pass [`ReaderDateRange::after`] as `before`. A site stream sends no
/// `date_range`, so pass its oldest post's date. A stream has ended when a page has no posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderPostsResponse")]
pub struct ReaderPostsResponse {
    /// The page's posts, without any in [`Self::unreadable_posts`].
    pub posts: Vec<ReaderPost>,
    /// The posts on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_posts: Vec<ReaderUnreadablePost>,
    /// The dates the page covers, in the date the stream is ordered by: `before` is the newest
    /// post's and `after` the oldest post's. That date is when a post was published, except on
    /// the Liked stream, where it's [`ReaderPost::date_liked`], and on Freshly Pressed, where it's
    /// [`ReaderPost::editorial_displayed_on`]. Site streams don't send it, and a page with no posts
    /// has no dates in it.
    #[uniffi(default = None)]
    pub date_range: Option<ReaderDateRange>,
    /// How many posts there are. Site streams send it.
    #[uniffi(default = None)]
    pub found: Option<u64>,
}

#[derive(Deserialize)]
struct RawReaderPostsResponse {
    #[serde(default)]
    posts: Option<LenientVec<ReaderPost>>,
    #[serde(default)]
    date_range: Option<ReaderDateRange>,
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    found: Option<u64>,
}

impl From<RawReaderPostsResponse> for ReaderPostsResponse {
    fn from(raw: RawReaderPostsResponse) -> Self {
        let (posts, unreadable_posts) = sort_posts(raw.posts);
        Self {
            posts,
            unreadable_posts,
            date_range: raw.date_range,
            found: raw.found,
        }
    }
}

/// Query parameters for the Reader streams that page by date: Following, a site's posts, and a
/// feed's posts. For the next page, pass the oldest post's date as `before`.
///
/// The other streams that page by date have parameters of their own: [`ReaderLikedPostsParams`],
/// [`ReaderOrganizationPostsParams`] and [`ReaderFreshlyPressedParams`].
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
    /// The fields of each post to return, comma-separated, such as `ID,site_ID,date`. Without it
    /// a post arrives with every field, about 60 of them.
    ///
    /// - `ID` has to be among them. A post without one can't be read, and is listed in
    ///   [`ReaderPostsResponse::unreadable_posts`].
    /// - `meta` has to be among them for what [`Self::meta`] asks for to arrive.
    /// - Paging by date needs `date`.
    /// - The API works some fields out from others, and only from the ones asked for. It fills
    ///   `featured_media` from a post's `attachments` and `content`, so without those it's empty.
    ///
    /// A stream with no posts then answers with an empty list where the page would be, which reads
    /// as a page with no posts.
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

/// The order of a Reader stream's posts.
///
/// The API reads the value in capitals only, and takes anything else as its default.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum ReaderStreamOrder {
    /// Newest first. The API's default.
    Desc,
    /// Oldest first. What that does depends on the stream: see the stream's parameters.
    Asc,
}

impl_as_query_value_from_to_string!(ReaderStreamOrder);

/// Query parameters for the posts the user liked.
///
/// The stream is ordered by when the user liked each post, newest like first, and `before` and
/// `after` compare with that time rather than with when a post was published. For the next page,
/// pass the page's [`ReaderDateRange::after`] as `before`: it's the last post's
/// [`ReaderPost::date_liked`]. The stream has ended when a page has no posts.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The stream pages by the date of the like rather than a page number.
pub struct ReaderLikedPostsParams {
    /// The number of posts to return: 10 by default, and at most 40.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Only return posts liked before this time. A post liked at that very second is left out.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts liked after this time. A post liked at that very second is left out.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// The order of the posts. [`ReaderStreamOrder::Asc`] starts from the oldest like, and a page's
    /// `date_range` then has the last post's date in `before`. To page that way, pass that date
    /// as `after`.
    #[uniffi(default = None)]
    pub order: Option<ReaderStreamOrder>,
    /// The page to return, counting from 1. Don't page with it: each page starts 5 posts later
    /// than it should, so those posts are never returned. Page with `before`.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Extra data to include, comma-separated, as in [`ReaderStreamParams::meta`].
    #[uniffi(default = None)]
    pub meta: Option<String>,
    /// The fields of each post to return, comma-separated, as in [`ReaderStreamParams::fields`].
    /// The page's `date_range` arrives whichever fields are asked for, and a stream with no posts
    /// still answers with a page, not an empty list.
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

/// Query parameters for the streams of an organization's posts: Automattic's, and the P2s the user
/// follows.
///
/// Both are ordered by when each post was published, newest first. For the next page, pass the
/// page's [`ReaderDateRange::after`], the last post's date, as `before`. The stream has ended when
/// a page has no posts.
///
/// The API declares the same parameters for both. What each one does was checked on the Automattic
/// stream only: the account used had no P2 posts.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The streams page by date. `page` is a parameter of its own.
pub struct ReaderOrganizationPostsParams {
    /// The number of posts to return: 10 by default, and at most 40.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Only return posts published before this time. A post published at that very second is left
    /// out.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts published after this time. A post published at that very second is left
    /// out.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// The order of the posts on the page. [`ReaderStreamOrder::Asc`] returns the same posts as
    /// [`ReaderStreamOrder::Desc`], the newest ones, with the oldest of them first. It doesn't
    /// start the stream from its oldest post.
    #[uniffi(default = None)]
    pub order: Option<ReaderStreamOrder>,
    /// The page to return, counting from 1, from the newest post at the time of the request. A
    /// new post moves every other post along by one, so page with `before` instead.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Extra data to include, comma-separated, as in [`ReaderStreamParams::meta`].
    #[uniffi(default = None)]
    pub meta: Option<String>,
    /// The fields of each post to return, comma-separated, as in [`ReaderStreamParams::fields`].
    /// The page's `date_range` arrives whichever fields are asked for, and a stream with no posts
    /// still answers with a page, not an empty list.
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

/// Query parameters for Freshly Pressed, the posts WordPress.com's editors picked.
///
/// The stream is ordered by when each post is shown, [`ReaderPost::editorial_displayed_on`], newest
/// first. That's about one post a day, and the newest can be a few hours in the future. `before`
/// and `after` compare with that time rather than with when a post was published. For the next
/// page, pass the page's [`ReaderDateRange::after`] as `before`. The stream has ended when a page
/// has no posts.
///
/// The API also takes `format=rss`, which redirects to an RSS feed, so it isn't here.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The stream pages by date rather than page number.
pub struct ReaderFreshlyPressedParams {
    /// The number of posts to return: 10 by default, and at most 40.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// Only return posts shown before this time. A post shown at that very second is left out.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts shown at or after this time.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// The width, in pixels, to size the images in each post's content for. 480 by default.
    #[uniffi(default = None)]
    pub content_width: Option<u32>,
    /// The width, in pixels, of the screenshot in `editorial.image`. 252 by default.
    #[uniffi(default = None)]
    pub thumb_width: Option<u32>,
    /// The height, in pixels, of thumbnail images. 160 by default. It changed nothing in the
    /// responses that were looked at.
    #[uniffi(default = None)]
    pub thumb_height: Option<u32>,
    /// Extra data to include, comma-separated, as in [`ReaderStreamParams::meta`].
    #[uniffi(default = None)]
    pub meta: Option<String>,
    /// The fields of each post to return, comma-separated, as in [`ReaderStreamParams::fields`].
    /// `editorial` has the time a post is shown. The page's `date_range` arrives whichever fields
    /// are asked for, and a stream with no posts still answers with a page, not an empty list.
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

/// How the results of a search of Reader posts are ordered.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReaderSearchSort {
    /// Best match first. The API's default. A search pages through its best 200 results and no
    /// further.
    Relevance,
    /// Newest first. A search pages on past 200 results.
    Date,
}

impl_as_query_value_from_to_string!(ReaderSearchSort);

/// Query parameters for a search of Reader posts.
///
/// For the next page, pass the page's [`ReaderSearchPostsResponse::next_page_handle`] as
/// `page_handle`, with the same `q` and `sort`. The search has ended when a page has no handle.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct ReaderSearchPostsParams {
    /// What to search for. Without it the API answers 400.
    pub q: String,
    /// The number of posts to return: 20 by default, and 20 at most. The API answers 400 to more.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// The previous page's [`ReaderSearchPostsResponse::next_page_handle`]. Send the `sort` that
    /// returned it: without it, the API ignores a handle from a search by date.
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
    /// How many results to skip: 199 at most, and the API answers 400 to more. A search by
    /// relevance pages this way, and its page handle holds the next offset.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// The order of the results. The API's default is [`ReaderSearchSort::Relevance`].
    #[uniffi(default = None)]
    pub sort: Option<ReaderSearchSort>,
    /// Only return posts published before this time.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts published after this time.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// The language to search in, such as `fr`.
    #[uniffi(default = None)]
    pub lang: Option<String>,
    /// The name of the algorithm to search with. The API sends back the name it's given, in
    /// [`ReaderSearchPostsResponse::algorithm`] and in the page handle. Another name changed
    /// nothing in the results that were looked at.
    #[uniffi(default = None)]
    pub algorithm: Option<String>,
    /// Extra data to include, comma-separated, as in [`ReaderStreamParams::meta`].
    #[uniffi(default = None)]
    pub meta: Option<String>,
    /// The fields of each post to return, comma-separated, as in [`ReaderStreamParams::fields`].
    /// A search with no results still answers with a page, not an empty list.
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

// Written out rather than derived: `q` is always sent, and the derive reads every field back
// from a query as an `Option`.
impl AppendUrlQueryPairs for ReaderSearchPostsParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_query_value_pair("q", &self.q)
            .append_option_query_value_pair("number", self.number.as_ref())
            .append_option_query_value_pair("page_handle", self.page_handle.as_ref())
            .append_option_query_value_pair("offset", self.offset.as_ref())
            .append_option_query_value_pair("sort", self.sort.as_ref())
            .append_option_query_value_pair("before", self.before.as_ref())
            .append_option_query_value_pair("after", self.after.as_ref())
            .append_option_query_value_pair("lang", self.lang.as_ref())
            .append_option_query_value_pair("algorithm", self.algorithm.as_ref())
            .append_option_query_value_pair("meta", self.meta.as_ref())
            .append_option_query_value_pair("fields", self.fields.as_ref());
    }
}

/// A post in the results of a search, with what only a search says about it.
///
/// A search sets [`ReaderPost::tagged_on`] to the time of the request, which says nothing about
/// the post.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSearchPost {
    #[serde(flatten)]
    pub post: ReaderPost,
    /// How well the post matches. A search by relevance is ordered by it, highest first, and a
    /// search by date doesn't send it.
    #[serde(default, deserialize_with = "deserialize_optional_score")]
    #[uniffi(default = None)]
    pub score: Option<f64>,
    /// How many followers the post's site has.
    #[serde(default, deserialize_with = "deserialize_u64_or_string_or_none")]
    #[uniffi(default = None)]
    pub site_follower_count: Option<u64>,
    /// When the search picked the post up, a few seconds after it was published. A search by date
    /// is ordered by it, newest first.
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    #[uniffi(default = None)]
    pub date_added: Option<WpGmtDateTime>,
}

/// A page of the results of a search of Reader posts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderSearchPostsResponse")]
pub struct ReaderSearchPostsResponse {
    /// The page's posts, without any in [`Self::unreadable_posts`]. A page of a search by date can
    /// have fewer than were asked for and not be the last.
    pub posts: Vec<ReaderSearchPost>,
    /// The posts on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_posts: Vec<ReaderUnreadablePost>,
    /// Pass it as `page_handle` for the next page. `None` on the last page. The API sends it as
    /// `next_page`.
    ///
    /// A search by relevance pages by offset, so a result that moves between two requests can
    /// arrive twice, or not at all.
    #[uniffi(default = None)]
    pub next_page_handle: Option<String>,
    /// The name of the algorithm that picked the results.
    #[uniffi(default = None)]
    pub algorithm: Option<String>,
    /// The [`ReaderSearchPost::date_added`] of two of the page's posts. Don't page with it: page
    /// with [`Self::next_page_handle`]. In a search by date, many posts share a second.
    #[uniffi(default = None)]
    pub date_range: Option<ReaderDateRange>,
}

#[derive(Deserialize)]
struct RawReaderSearchPostsResponse {
    #[serde(default)]
    posts: Option<LenientVec<ReaderSearchPost>>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    next_page: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    algorithm: Option<String>,
    #[serde(default)]
    date_range: Option<ReaderDateRange>,
}

impl From<RawReaderSearchPostsResponse> for ReaderSearchPostsResponse {
    fn from(raw: RawReaderSearchPostsResponse) -> Self {
        let (posts, unreadable_posts) = sort_posts(raw.posts);
        Self {
            posts,
            unreadable_posts,
            next_page_handle: raw.next_page,
            algorithm: raw.algorithm,
            date_range: raw.date_range,
        }
    }
}

wp_content_string_id!(ReaderListOwner);
wp_content_string_id!(ReaderListSlug);

/// Query parameters for a Reader list's posts. For the next page, pass the page's
/// [`ReaderListPostsResponse::next_page_handle`] as `page_handle`.
///
/// The API declares `page`, `order`, `before` and `after` too. None of them changed anything in
/// the responses that were looked at, and neither did `meta`. `fields` empties the response.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The stream pages with `page_handle` rather than a page number.
pub struct ReaderListPostsParams {
    /// The most posts to return: 10 by default. A page had 197 at most, whatever was asked for,
    /// and can have far fewer than were asked for without being the last.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// The previous page's [`ReaderListPostsResponse::next_page_handle`].
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
    /// The page to return, counting from 1. It changed nothing in the responses that were looked
    /// at.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// The order of the posts. It changed nothing in the responses that were looked at.
    #[uniffi(default = None)]
    pub order: Option<ReaderStreamOrder>,
    /// Only return posts published before this time. It changed nothing in the responses that
    /// were looked at.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts published after this time. It changed nothing in the responses that were
    /// looked at.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
}

/// A page of a Reader list's posts, newest first.
///
/// The API sends the page as cards, each holding a post. This is their posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderListPostsResponse")]
pub struct ReaderListPostsResponse {
    /// The page's posts, without any in [`Self::unreadable_posts`]. Empty for a list with nothing
    /// in it.
    pub posts: Vec<ReaderPost>,
    /// The cards on the page that couldn't be read as a post. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_posts: Vec<ReaderUnreadablePost>,
    /// How many of the page's cards came without a post. The API sends `false` where a post would
    /// be, and often: for 11 of the 40 cards on one page that was looked at.
    #[uniffi(default = 0)]
    pub missing_posts: u32,
    /// Pass it as `page_handle` for the next page. `None` when there isn't one.
    #[uniffi(default = None)]
    pub next_page_handle: Option<String>,
}

#[derive(Deserialize)]
struct RawReaderListPostsResponse {
    #[serde(default)]
    cards: Option<LenientVec<RawCard>>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    next_page_handle: Option<String>,
}

/// A card as it arrives: its kind, and its data still as JSON, to read once the kind is known.
#[derive(Deserialize)]
struct RawCard {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    data: Option<Box<RawValue>>,
}

impl RawCard {
    /// The card's data, unless the API sent `false` or `null` where the data would be.
    fn data(&self) -> Option<&RawValue> {
        self.data
            .as_deref()
            .filter(|data| !matches!(data.get(), "false" | "null"))
    }
}

impl From<RawReaderListPostsResponse> for ReaderListPostsResponse {
    fn from(raw: RawReaderListPostsResponse) -> Self {
        let (cards, mut unreadable_posts) = sort_posts(raw.cards);
        let mut posts = Vec::new();
        let mut missing_posts = 0;
        for card in cards {
            let kind = card.kind.as_deref().unwrap_or_default();
            if kind != "post" {
                let error = format!("expected a `post` card, not a `{kind}` card");
                unreadable_posts.push(ReaderUnreadablePost {
                    identity: None,
                    error,
                });
                continue;
            }
            let Some(data) = card.data() else {
                missing_posts += 1;
                continue;
            };
            match serde_json::from_str::<ReaderPost>(data.get()) {
                Ok(post) => posts.push(post),
                Err(error) => unreadable_posts.push(unreadable_post(data, error.to_string())),
            }
        }
        Self {
            posts,
            unreadable_posts,
            missing_posts,
            next_page_handle: raw.next_page_handle,
        }
    }
}

wp_content_string_id!(ReaderTagSlug);

/// How a tag's posts are ordered.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReaderTagPostsSort {
    /// Newest first. The API's default.
    Date,
    /// Most popular first. A page then has no [`ReaderTagPostsResponse::date_range`].
    Popularity,
}

impl_as_query_value_from_to_string!(ReaderTagPostsSort);

/// Query parameters for a tag's posts.
///
/// Sorted by date, pass the page's [`ReaderTagPostsResponse::next_page_handle`] as `page_handle`
/// for the next page. Sorted by popularity, pass the next `page` instead: the API answers a
/// handle from that order with the first page again.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // `page` is a parameter of its own: the stream has no page count.
pub struct ReaderTagPostsParams {
    /// The number of posts to return. The API returns 10 at most, whatever is asked for.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// The previous page's [`ReaderTagPostsResponse::next_page_handle`], from a page sorted by
    /// date. A handle from a page sorted by popularity returns the first page again.
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
    /// The page to return, counting from 1. It works in either order, and it's the only way to
    /// page posts sorted by popularity.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// The order of the posts. The API's default is [`ReaderTagPostsSort::Date`].
    #[uniffi(default = None)]
    pub sort: Option<ReaderTagPostsSort>,
    /// Only return posts published before this time.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts published after this time.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
}

/// A page of the posts of a tag, or of several tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderTagPostsResponse")]
pub struct ReaderTagPostsResponse {
    /// The page's posts, without any in [`Self::unreadable_posts`]. Empty for a tag nobody has
    /// used: the API has no "not found" for a tag.
    pub posts: Vec<ReaderPost>,
    /// The posts on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_posts: Vec<ReaderUnreadablePost>,
    /// Pass it as `page_handle` for the next page of posts sorted by date. `None` on the last
    /// page. Posts sorted by popularity page by `page`: their handle returns the first page again.
    #[uniffi(default = None)]
    pub next_page_handle: Option<String>,
    /// The dates the page covers. Missing when the posts are sorted by popularity.
    #[uniffi(default = None)]
    pub date_range: Option<ReaderDateRange>,
}

#[derive(Deserialize)]
struct RawReaderTagPostsResponse {
    #[serde(default)]
    posts: Option<LenientVec<ReaderPost>>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    next_page_handle: Option<String>,
    #[serde(default)]
    date_range: Option<ReaderDateRange>,
}

impl From<RawReaderTagPostsResponse> for ReaderTagPostsResponse {
    fn from(raw: RawReaderTagPostsResponse) -> Self {
        let (posts, unreadable_posts) = sort_posts(raw.posts);
        Self {
            posts,
            unreadable_posts,
            next_page_handle: raw.next_page_handle,
            date_range: raw.date_range,
        }
    }
}

/// Query parameters for the posts of several tags. With one tag, it's that tag's posts.
///
/// It pages as a tag's posts do. Sorted by date, pass the page's
/// [`ReaderTagPostsResponse::next_page_handle`] as `page_handle` for the next page. Sorted by
/// popularity, pass the next `page` instead: the API answers a handle from that order with the
/// first page again.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // `page` is a parameter of its own: the stream has no page count.
pub struct ReaderTagsPostsParams {
    /// The tags to return posts for. Without any, the API answers 400.
    pub tags: Vec<ReaderTagSlug>,
    /// The number of posts to return. The API returns 10 at most, whatever is asked for.
    #[uniffi(default = None)]
    pub number: Option<u32>,
    /// The previous page's [`ReaderTagPostsResponse::next_page_handle`], from a page sorted by
    /// date. A handle from a page sorted by popularity returns the first page again.
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
    /// The page to return, counting from 1. It works in either order, and it's the only way to
    /// page posts sorted by popularity.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// The order of the posts. The API's default is [`ReaderTagPostsSort::Date`].
    #[uniffi(default = None)]
    pub sort: Option<ReaderTagPostsSort>,
    /// Only return posts published before this time.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Only return posts published after this time.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
}

/// A card in a stream of cards, such as Discover.
///
/// A stream of cards mixes posts with other things to show between them. Each kind the API is
/// known to send has a case. A card of any other kind is [`ReaderCard::Unknown`], so that a new
/// kind doesn't fail the page.
#[derive(Debug, Clone, PartialEq, Serialize, uniffi::Enum)]
// A stream is nearly all posts, so boxing the post would cost an allocation on most cards.
#[allow(clippy::large_enum_variant)]
pub enum ReaderCard {
    /// A post. The API calls the card `post`.
    Post { post: ReaderPost },
    /// Sites to recommend. The API calls the card `recommended_blogs`. Discover sends one on each
    /// page, at any position, with up to as many sites as the cards that were asked for.
    RecommendedSites { sites: Vec<ReaderCardSite> },
    /// Sites that have just started. The API calls the card `new_sites`. The first posts stream
    /// sends one at the end of each page, and it can be empty.
    NewSites { sites: Vec<ReaderCardSite> },
    /// Tags the user may like. The API calls the card `interests_you_may_like`. Discover sends one
    /// at the end of a page that was asked for with `tags`.
    SuggestedTags { tags: Vec<ReaderCardTag> },
    /// A card of a kind this library doesn't know, with the name the API gave it.
    Unknown { card_type: String },
}

/// A site on a card of recommended or new sites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderCardSite")]
pub struct ReaderCardSite {
    #[uniffi(default = None)]
    pub site_id: Option<WpComSiteId>,
    /// `None` for a site the Reader has no feed for.
    #[uniffi(default = None)]
    pub feed_id: Option<ReaderFeedId>,
    #[uniffi(default = None)]
    pub name: Option<String>,
    #[uniffi(default = None)]
    pub description: Option<String>,
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[uniffi(default = None)]
    pub feed_url: Option<String>,
    /// The site's icon: `icon.img`, else `icon.ico`. Many sites have none.
    #[uniffi(default = None)]
    pub icon_url: Option<String>,
    #[uniffi(default = None)]
    pub is_private: Option<bool>,
    #[uniffi(default = None)]
    pub is_jetpack: Option<bool>,
    /// Whether to show the site as its feed rather than as a WordPress.com site.
    #[uniffi(default = None)]
    pub prefer_feed: Option<bool>,
    #[uniffi(default = None)]
    pub subscribers_count: Option<u64>,
}

#[derive(Deserialize)]
struct RawReaderCardSite {
    #[serde(
        rename = "ID",
        default,
        deserialize_with = "deserialize_u64_or_string_or_none_as_t"
    )]
    site_id: Option<WpComSiteId>,
    #[serde(
        rename = "feed_ID",
        default,
        deserialize_with = "deserialize_u64_or_string_or_none_as_t"
    )]
    feed_id: Option<ReaderFeedId>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    description: Option<String>,
    #[serde(
        rename = "URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    url: Option<String>,
    #[serde(
        rename = "feed_URL",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    feed_url: Option<String>,
    #[serde(default, deserialize_with = "deserialize_object_or_empty")]
    icon: Option<RawSiteIcon>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    is_private: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    jetpack: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    prefer_feed: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    subscribers_count: Option<u64>,
}

impl From<RawReaderCardSite> for ReaderCardSite {
    fn from(raw: RawReaderCardSite) -> Self {
        Self {
            site_id: raw.site_id,
            feed_id: raw.feed_id,
            name: raw.name,
            description: raw.description,
            url: raw.url,
            feed_url: raw.feed_url,
            icon_url: raw.icon.and_then(|icon| icon.img.or(icon.ico)),
            is_private: raw.is_private,
            is_jetpack: raw.jetpack,
            prefer_feed: raw.prefer_feed,
            subscribers_count: raw.subscribers_count,
        }
    }
}

/// A tag on a card of tags the user may like.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderCardTag {
    pub slug: ReaderTagSlug,
    /// The tag's name, to show.
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    #[uniffi(default = None)]
    pub title: Option<String>,
    /// How strongly the tag is suggested. A card's tags arrive strongest first.
    #[serde(default, deserialize_with = "deserialize_optional_score")]
    #[uniffi(default = None)]
    pub score: Option<f64>,
}

/// A card on a page that couldn't be read, or part of one.
///
/// One card the library can't read doesn't fail its page. The page keeps the cards it can read,
/// and lists the others as these. A site or a tag that can't be read is listed too, and its card
/// keeps the rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderUnreadableCard {
    /// The card's kind, as the API names it. `None` when that couldn't be read either.
    #[uniffi(default = None)]
    pub card_type: Option<String>,
    /// Which post it is, for a post card whose IDs could be read.
    #[uniffi(default = None)]
    pub post: Option<ReaderPostSaveIdentity>,
    /// Why it couldn't be read. A line and column in it count from the start of the card, or of
    /// the post, site or tag in it.
    pub error: String,
}

/// Reads a page's cards, and lists the ones that can't be read.
fn sort_cards(cards: Option<LenientVec<RawCard>>) -> (Vec<ReaderCard>, Vec<ReaderUnreadableCard>) {
    let cards = cards.unwrap_or_default();
    let mut unreadable: Vec<_> = cards
        .invalid
        .into_iter()
        .map(|card| ReaderUnreadableCard {
            card_type: None,
            post: None,
            error: card.error.to_string(),
        })
        .collect();
    let mut read = Vec::new();
    for card in cards.valid {
        let Some(kind) = card.kind.clone() else {
            unreadable.push(ReaderUnreadableCard {
                card_type: None,
                post: None,
                error: "the card has no `type`".to_string(),
            });
            continue;
        };
        match card.read(&kind, &mut unreadable) {
            Ok(card) => read.push(card),
            Err(error) => unreadable.push(error),
        }
    }
    (read, unreadable)
}

impl RawCard {
    /// The card, once its kind says how to read its data. A site or a tag in it that can't be
    /// read goes to `unreadable`, and the card keeps the rest.
    fn read(
        &self,
        kind: &str,
        unreadable: &mut Vec<ReaderUnreadableCard>,
    ) -> Result<ReaderCard, ReaderUnreadableCard> {
        match kind {
            "post" => {
                let data = self
                    .data()
                    .ok_or_else(|| unreadable_card(kind, None, "the card has no post"))?;
                serde_json::from_str(data.get())
                    .map(|post| ReaderCard::Post { post })
                    .map_err(|error| {
                        unreadable_card(kind, serde_json::from_str(data.get()).ok(), error)
                    })
            }
            "recommended_blogs" => Ok(ReaderCard::RecommendedSites {
                sites: read_card_items(kind, self.list(kind)?, unreadable),
            }),
            "new_sites" => Ok(ReaderCard::NewSites {
                sites: read_card_items(kind, self.list(kind)?, unreadable),
            }),
            "interests_you_may_like" => Ok(ReaderCard::SuggestedTags {
                tags: read_card_items(kind, self.list(kind)?, unreadable),
            }),
            _ => Ok(ReaderCard::Unknown {
                card_type: kind.to_string(),
            }),
        }
    }

    /// The card's data as a list, each element still as JSON. A card without data has none.
    fn list(&self, kind: &str) -> Result<Vec<Box<RawValue>>, ReaderUnreadableCard> {
        match self.data() {
            None => Ok(Vec::new()),
            Some(data) => {
                serde_json::from_str(data.get()).map_err(|error| unreadable_card(kind, None, error))
            }
        }
    }
}

fn unreadable_card(
    kind: &str,
    post: Option<ReaderPostSaveIdentity>,
    error: impl fmt::Display,
) -> ReaderUnreadableCard {
    ReaderUnreadableCard {
        card_type: Some(kind.to_string()),
        post,
        error: error.to_string(),
    }
}

/// The sites or tags of a card that can be read. Each of the others is listed in `unreadable`.
fn read_card_items<T: DeserializeOwned>(
    kind: &str,
    items: Vec<Box<RawValue>>,
    unreadable: &mut Vec<ReaderUnreadableCard>,
) -> Vec<T> {
    items
        .into_iter()
        .filter_map(|item| match serde_json::from_str(item.get()) {
            Ok(item) => Some(item),
            Err(error) => {
                unreadable.push(unreadable_card(kind, None, error));
                None
            }
        })
        .collect()
}

/// How Discover picks its posts.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReaderDiscoverSort {
    /// Popular posts first. The API's default. The first pages are recent posts, and later ones
    /// reach back years.
    Popular,
    /// Newest first. The API sorts this way for any value but `popular`.
    Latest,
}

impl_as_query_value_from_to_string!(ReaderDiscoverSort);

/// Query parameters for Discover's stream of cards. For the next page, pass the page's
/// [`ReaderDiscoverResponse::next_page_handle`] as `page_handle`. The stream has ended when a
/// page has no handle.
///
/// `meta` and `fields` aren't here: `meta` adds nothing to a post, and `fields` trims a post only
/// part of the way.
#[derive(Debug, Default, PartialEq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The stream pages with `page_handle` rather than a page number.
pub struct ReaderDiscoverParams {
    /// The number of cards to return: 8 by default. The card of recommended sites is one of them.
    /// The API returned as many as 120 when asked, and 86 when asked for 150 or more. It answers
    /// 500 to `0`.
    #[uniffi(default = None)]
    pub count: Option<u32>,
    /// The previous page's [`ReaderDiscoverResponse::next_page_handle`]. The API answers 500 to a
    /// handle it didn't send.
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
    /// Tags to lean the stream toward. With any, the page also ends with a card of tags the user
    /// may like, which isn't counted in `count`.
    #[uniffi(default = [])]
    pub tags: Vec<ReaderTagSlug>,
    /// How the posts are picked. The API's default is [`ReaderDiscoverSort::Popular`].
    #[uniffi(default = None)]
    pub sort: Option<ReaderDiscoverSort>,
    /// How much to hold a post's age against it, between 0 and 1. It changed nothing in the
    /// responses that were looked at.
    #[uniffi(default = None)]
    pub age_based_decay: Option<f64>,
}

/// A page of Discover's stream of cards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderDiscoverResponse")]
pub struct ReaderDiscoverResponse {
    /// The page's cards, in the order to show them, without any in [`Self::unreadable_cards`].
    pub cards: Vec<ReaderCard>,
    /// The cards on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_cards: Vec<ReaderUnreadableCard>,
    /// Pass it as `page_handle` for the next page. `None` on the last page, which has a card of
    /// recommended sites and no posts.
    #[uniffi(default = None)]
    pub next_page_handle: Option<String>,
    /// The tags the user follows, or the API's own choice for a signed-out request. The last page
    /// doesn't send them.
    #[uniffi(default = [])]
    pub user_interests: Vec<ReaderTagSlug>,
}

#[derive(Deserialize)]
struct RawReaderDiscoverResponse {
    #[serde(default)]
    cards: Option<LenientVec<RawCard>>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    next_page_handle: Option<String>,
    #[serde(default)]
    user_interests: Option<LenientVec<ReaderTagSlug>>,
}

impl From<RawReaderDiscoverResponse> for ReaderDiscoverResponse {
    fn from(raw: RawReaderDiscoverResponse) -> Self {
        let (cards, unreadable_cards) = sort_cards(raw.cards);
        Self {
            cards,
            unreadable_cards,
            next_page_handle: raw.next_page_handle,
            user_interests: raw.user_interests.unwrap_or_default().valid,
        }
    }
}

/// Query parameters for the stream of new sites' first posts. For the next page, pass the page's
/// [`ReaderFirstPostsResponse::next_page_handle`] as `page_handle`.
///
/// `meta` and `fields` aren't here: `meta` adds nothing to a post, and `fields` trims a post only
/// part of the way.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // The stream pages with `page_handle` rather than a page number.
pub struct ReaderFirstPostsParams {
    /// The number of cards to ask for: 8 by default. A page has fewer posts than that, often half
    /// as many. Asked for 16 or more, the API returns no posts and no handle, and it answers 500
    /// to `0`.
    #[uniffi(default = None)]
    pub count: Option<u32>,
    /// The previous page's [`ReaderFirstPostsResponse::next_page_handle`]. The API answers a
    /// handle it didn't send with the first page.
    #[uniffi(default = None)]
    pub page_handle: Option<String>,
}

/// A page of the stream of new sites' first posts: post cards, then a card of new sites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderFirstPostsResponse")]
pub struct ReaderFirstPostsResponse {
    /// The page's cards, in the order to show them, without any in [`Self::unreadable_cards`].
    pub cards: Vec<ReaderCard>,
    /// The cards on the page that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_cards: Vec<ReaderUnreadableCard>,
    /// Pass it as `page_handle` for the next page. `None` when there isn't one.
    #[uniffi(default = None)]
    pub next_page_handle: Option<String>,
}

#[derive(Deserialize)]
struct RawReaderFirstPostsResponse {
    #[serde(default)]
    cards: Option<LenientVec<RawCard>>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    next_page_handle: Option<String>,
}

impl From<RawReaderFirstPostsResponse> for ReaderFirstPostsResponse {
    fn from(raw: RawReaderFirstPostsResponse) -> Self {
        let (cards, unreadable_cards) = sort_cards(raw.cards);
        Self {
            cards,
            unreadable_cards,
            next_page_handle: raw.next_page_handle,
        }
    }
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
    #[serde(default, deserialize_with = "deserialize_tags")]
    tags: Option<Vec<ReaderPostTag>>,
    #[serde(default, deserialize_with = "deserialize_metadata")]
    metadata: Vec<RawMetadata>,
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

/// `editorial`, which Freshly Pressed sends for a post an editor picked. Only its image and the
/// time it's shown are kept.
#[derive(Deserialize)]
struct RawEditorial {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    image: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_wp_gmt_date_time")]
    displayed_on: Option<WpGmtDateTime>,
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
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    comment_count: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_bool_or_int")]
    comments_open: Option<bool>,
}

impl TryFrom<RawReaderPost> for ReaderPost {
    type Error = String;

    fn try_from(raw: RawReaderPost) -> Result<Self, Self::Error> {
        let metadata = raw.metadata;
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
        let editorial_image = raw
            .editorial
            .as_ref()
            .and_then(|editorial| editorial.image.as_deref());
        let display_image_url = reader_post_display_image_url(
            editorial_image,
            raw.featured_image.as_deref(),
            raw.featured_media.as_ref(),
        );
        let editorial_image_url =
            reader_post_editorial_image_url(editorial_image).map(str::to_string);
        let editorial_displayed_on = raw
            .editorial
            .as_ref()
            .and_then(|editorial| editorial.displayed_on);
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
            editorial_image_url,
            editorial_displayed_on,
            tags: raw.tags,
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

/// A count of something. The API sends a negative number for some posts' comments, which isn't a
/// count, so anything that isn't one reads as `None` rather than failing the post.
fn deserialize_optional_count<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_id(deserializer)
}

/// A width or height. The API sends a negative number for some images, which isn't a size, so
/// anything that isn't one reads as `None` rather than failing the post.
fn deserialize_optional_size<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_id(deserializer)
}

/// A search score, which the API sends as a number. A number in a string is read too, and
/// anything else reads as `None` rather than failing the post.
fn deserialize_optional_score<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::Number(number)) => number.as_f64(),
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

/// `tags`, an object of tags by slug, in the order the API sends them, which is the order the web
/// Reader shows them in. Going through `serde_json::Value` would sort them by slug. With no tags,
/// it's `[]`, and the API sends that as an empty object or array, `""`, `false` or `null`. Only a
/// response without the key at all reads as `None`.
///
/// A tag without a slug or a name is left out, so one malformed tag doesn't fail the post, and with
/// it a whole page of a stream.
fn deserialize_tags<'de, D>(deserializer: D) -> Result<Option<Vec<ReaderPostTag>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct TagsVisitor;

    impl<'de> Visitor<'de> for TagsVisitor {
        type Value = Vec<ReaderPostTag>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an object of tags by slug")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut tags = Vec::new();
            while let Some((_, tag)) = map.next_entry::<IgnoredAny, serde_json::Value>()? {
                tags.extend(serde_json::from_value::<ReaderPostTag>(tag).ok());
            }
            Ok(tags)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut tags = Vec::new();
            while let Some(tag) = seq.next_element::<serde_json::Value>()? {
                tags.extend(serde_json::from_value::<ReaderPostTag>(tag).ok());
            }
            Ok(tags)
        }

        fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
            if text.is_empty() {
                Ok(Vec::new())
            } else {
                Err(E::invalid_value(serde::de::Unexpected::Str(text), &self))
            }
        }

        fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }
    }

    deserializer.deserialize_any(TagsVisitor).map(Some)
}

/// `metadata`, a list of entries. PHP sends a list as an object when its keys aren't in sequence,
/// so an object's values are read as the list. With none, it's empty, and the API sends that as an
/// empty object or array, `""`, `false` or `null`.
fn deserialize_metadata<'de, D>(deserializer: D) -> Result<Vec<RawMetadata>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let entries = match value {
        Some(serde_json::Value::Array(entries)) => entries,
        Some(serde_json::Value::Object(fields)) => {
            fields.into_iter().map(|(_, entry)| entry).collect()
        }
        None | Some(serde_json::Value::Null) | Some(serde_json::Value::Bool(false)) => Vec::new(),
        Some(serde_json::Value::String(text)) if text.is_empty() => Vec::new(),
        Some(other) => {
            return Err(serde::de::Error::custom(format!(
                "expected a list of metadata, not {other}"
            )));
        }
    };
    entries
        .into_iter()
        .map(|entry| serde_json::from_value(entry).map_err(serde::de::Error::custom))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::posts::PostId;
    use crate::wp_com::reader_saved::{
        ReaderFeedId, ReaderFeedItemId, ReaderSavedKey, reader_saved_key_for_post,
    };
    use rstest::rstest;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> ReaderPost {
        serde_json::from_value(value).expect("the post parses")
    }

    /// A recording of a tag's posts on WordPress.com, trimmed, with the site, author and post
    /// details replaced. It keeps the types the API sent, such as `like_count` as a string.
    #[test]
    fn test_deserialize_tag_posts() {
        let json = include_str!("../../tests/wpcom/reader_posts/tag_posts.json");
        let page: ReaderTagPostsResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(page.posts.len(), 2);
        assert_eq!(
            page.next_page_handle.as_deref(),
            Some("ZnJvbT0yJmJlZm9yZT0yMDI2LTExLTE3VDE5JTNBMzklM0ExNC0wNSUzQTAw")
        );
        let range = page.date_range.expect("a page sorted by date has a range");
        assert!(range.before.is_some());
        assert!(range.after.is_some());

        let post = &page.posts[0];
        assert_eq!(post.identity.id, 1001);
        assert_eq!(post.identity.site_id, Some(WpComSiteId(2001)));
        assert!(!post.identity.is_external);
        assert_eq!(post.title.as_deref(), Some("Example post 1"));
        assert!(post.like_count.is_some());
        assert_eq!(post.tags.as_ref().map(Vec::len), Some(1));
    }

    /// What the API sends for a tag nobody has used: no posts, and nulls where a page has values.
    #[test]
    fn test_deserialize_tag_posts_for_an_unused_tag() {
        let page: ReaderTagPostsResponse = serde_json::from_value(json!({
            "success": true,
            "tags": ["no-such-tag"],
            "sort": "date",
            "lang": "en",
            "page": 1,
            "posts": [],
            "next_page_handle": null,
            "date_range": { "before": null, "after": null },
        }))
        .expect("the page parses");

        assert!(page.posts.is_empty());
        assert_eq!(page.next_page_handle, None);
        assert_eq!(
            page.date_range,
            Some(ReaderDateRange {
                before: None,
                after: None,
                unreadable_before: None,
                unreadable_after: None,
            })
        );
    }

    /// Sorted by popularity, a page has no `date_range`.
    #[test]
    fn test_deserialize_tag_posts_sorted_by_popularity() {
        let page: ReaderTagPostsResponse = serde_json::from_value(json!({
            "success": true,
            "tags": ["photography"],
            "sort": "popularity",
            "lang": "en",
            "page": 1,
            "posts": [{ "ID": 1, "site_ID": 2 }],
            "next_page_handle": "ZnJvbT0z",
        }))
        .expect("the page parses");

        assert_eq!(page.posts.len(), 1);
        assert_eq!(page.next_page_handle.as_deref(), Some("ZnJvbT0z"));
        assert_eq!(page.date_range, None);
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
        assert!(post.tags.as_ref().is_some_and(|tags| !tags.is_empty()));
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
        assert_eq!(post.tags, Some(vec![]));
        assert_eq!(post.site_locale, None, "Not asked for with meta=site");
    }

    /// A recording of the Liked stream on WordPress.com, trimmed, with the site, author and post
    /// details replaced. Its `date_range` is in the times the posts were liked.
    #[test]
    fn test_deserialize_liked_posts() {
        let json = include_str!("../../tests/wpcom/reader_posts/liked.json");
        let page: ReaderPostsResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(page.posts.len(), 2);
        assert!(page.unreadable_posts.is_empty());
        let liked: Vec<_> = page.posts.iter().map(|post| post.date_liked).collect();
        assert!(liked.iter().all(Option::is_some));
        let range = page.date_range.expect("the page has a range");
        assert_eq!(range.before, liked[0]);
        assert_eq!(range.after, liked[1], "The `before` of the next page");
        assert_ne!(
            range.after, page.posts[1].date,
            "Liked after it was published"
        );
        assert_eq!(page.posts[0].i_like, Some(true));
    }

    /// A recording of Freshly Pressed on WordPress.com, trimmed, with the site, author and post
    /// details replaced. Its `date_range` is in the times the posts are shown.
    #[test]
    fn test_deserialize_freshly_pressed() {
        let json = include_str!("../../tests/wpcom/reader_posts/freshly_pressed.json");
        let page: ReaderPostsResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(page.posts.len(), 2);
        assert!(page.unreadable_posts.is_empty());
        let shown: Vec<_> = page
            .posts
            .iter()
            .map(|post| post.editorial_displayed_on)
            .collect();
        assert!(shown.iter().all(Option::is_some));
        let range = page.date_range.expect("the page has a range");
        assert_eq!(range.before, shown[0]);
        assert_eq!(range.after, shown[1], "The `before` of the next page");
        assert_ne!(
            range.after, page.posts[1].date,
            "Shown after it was published"
        );

        let post = &page.posts[0];
        assert_eq!(post.identity.id, 1001);
        assert_eq!(
            post.editorial_image_url, None,
            "The pick is a screenshot of the page"
        );
        let railcar: serde_json::Value =
            serde_json::from_str(post.railcar.as_deref().expect("a railcar")).unwrap();
        assert_eq!(
            railcar["fetch_algo"],
            "read:recommendations:posts/freshly-pressed"
        );
    }

    /// What a stream that pages by date sends past its last post. Freshly Pressed sends an empty
    /// list for the range, and the others a range of empty strings.
    #[rstest]
    #[case::empty_dates(json!({ "date_range": { "before": "", "after": "" }, "number": 0, "posts": [] }))]
    #[case::empty_list(json!({ "date_range": [], "number": 0, "posts": [] }))]
    fn test_a_page_past_the_end_of_a_stream(#[case] page: serde_json::Value) {
        let page: ReaderPostsResponse =
            serde_json::from_str(&page.to_string()).expect("the page parses");
        assert!(page.posts.is_empty());
        assert!(page.unreadable_posts.is_empty());
        assert_eq!(
            page.date_range,
            Some(ReaderDateRange {
                before: None,
                after: None,
                unreadable_before: None,
                unreadable_after: None,
            })
        );
    }

    #[rstest]
    #[case::shown(json!({ "displayed_on": "2026-11-19T12:00:00+00:00" }), Some("2026-11-19T12:00:00+00:00"))]
    #[case::empty(json!({ "displayed_on": "" }), None)]
    #[case::left_out(json!({ "image": "https://editorial/img.jpg" }), None)]
    #[case::no_editorial(json!(null), None)]
    fn test_editorial_displayed_on(
        #[case] editorial: serde_json::Value,
        #[case] expected: Option<&str>,
    ) {
        let post = parse(json!({ "ID": 1, "site_ID": 2, "editorial": editorial }));
        assert_eq!(
            post.editorial_displayed_on,
            expected.map(|date| date.parse().unwrap())
        );
    }

    /// A recording of a search on WordPress.com, trimmed, with the site, author and post details
    /// replaced.
    #[test]
    fn test_deserialize_search() {
        let json = include_str!("../../tests/wpcom/reader_posts/search.json");
        let page: ReaderSearchPostsResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(page.posts.len(), 2);
        assert!(page.unreadable_posts.is_empty());
        assert_eq!(
            page.next_page_handle.as_deref(),
            Some("offset=2&algorithm=read:search:posts/11")
        );
        assert_eq!(page.algorithm.as_deref(), Some("read:search:posts/11"));

        let first = &page.posts[0];
        assert_eq!(first.post.identity.id, 1001);
        assert_eq!(first.post.title.as_deref(), Some("Example post 1"));
        assert_eq!(first.score, Some(42.5));
        assert_eq!(first.site_follower_count, Some(1));
        assert_eq!(first.post.comment_count, Some(1));
        // The API sends the time without an offset. It's GMT.
        assert_eq!(
            first.date_added,
            Some("2026-11-17T12:00:00+00:00".parse().unwrap())
        );

        // The range is in the same time, taken from the first and last posts.
        let range = page.date_range.expect("the page has a range");
        assert_eq!(range.after, first.date_added);
        assert_eq!(range.before, page.posts[1].date_added);
    }

    /// A page of a search by date: its handle isn't an offset, and its posts have no score.
    #[test]
    fn test_deserialize_search_by_date() {
        let page: ReaderSearchPostsResponse = serde_json::from_value(json!({
            "date_range": { "before": "2026-11-19 05:41:42", "after": "2026-11-19 05:35:52" },
            "posts": [
                { "ID": 1, "site_ID": 10, "date_added": "2026-11-19 05:41:42" },
                { "ID": 2, "site_ID": 11, "date_added": "2026-11-19 05:35:52" },
            ],
            "algorithm": "read:search:posts/11",
            "next_page": "algorithm=read:search:posts/11&ts=1795066552&blog=11&post=2",
        }))
        .expect("the page parses");

        assert_eq!(
            page.next_page_handle.as_deref(),
            Some("algorithm=read:search:posts/11&ts=1795066552&blog=11&post=2")
        );
        assert!(page.posts.iter().all(|post| post.score.is_none()));
        assert!(page.posts.iter().all(|post| post.date_added.is_some()));
    }

    /// What a search with no results sends: no posts, no handle, and a range of empty strings.
    #[test]
    fn test_deserialize_search_with_no_results() {
        let page: ReaderSearchPostsResponse = serde_json::from_value(json!({
            "date_range": { "before": "", "after": "" },
            "posts": [],
            "algorithm": "read:search:posts/11",
        }))
        .expect("the page parses");

        assert!(page.posts.is_empty());
        assert_eq!(page.next_page_handle, None);
        assert_eq!(page.date_range.and_then(|range| range.before), None);
    }

    #[rstest]
    #[case::number(json!(4443.716), Some(4443.716))]
    #[case::whole_number(json!(12), Some(12.0))]
    #[case::string(json!("3383.948"), Some(3383.948))]
    #[case::not_a_number(json!("high"), None)]
    #[case::null(json!(null), None)]
    #[case::false_(json!(false), None)]
    fn test_search_score(#[case] score: serde_json::Value, #[case] expected: Option<f64>) {
        let post: ReaderSearchPost =
            serde_json::from_value(json!({ "ID": 1, "site_ID": 2, "score": score })).unwrap();
        assert_eq!(post.score, expected);
    }

    #[test]
    fn test_a_page_of_a_search_keeps_its_posts_when_one_cannot_be_read() {
        let page: ReaderSearchPostsResponse = serde_json::from_value(json!({
            "posts": [
                { "ID": 1, "site_ID": 10, "score": 2.5 },
                { "ID": 2, "site_ID": 10, "date": UNREADABLE_DATE },
            ],
            "next_page": "offset=2",
        }))
        .expect("the page parses");

        assert_eq!(page.posts.len(), 1);
        assert_eq!(page.posts[0].post.identity.id, 1);
        assert_eq!(page.next_page_handle.as_deref(), Some("offset=2"));
        let unreadable: Vec<_> = page
            .unreadable_posts
            .iter()
            .map(|post| post.identity.as_ref().map(|identity| identity.id))
            .collect();
        assert_eq!(unreadable, [Some(2)]);
    }

    /// A recording of a list's posts on WordPress.com, trimmed, with the site, author and post
    /// details replaced. The second of its three cards came without a post.
    #[test]
    fn test_deserialize_list_posts() {
        let json = include_str!("../../tests/wpcom/reader_posts/list_posts.json");
        let page: ReaderListPostsResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(ids(&page.posts), [1001, 1002]);
        assert!(page.unreadable_posts.is_empty());
        assert_eq!(page.missing_posts, 1);
        assert_eq!(
            page.next_page_handle.as_deref(),
            Some("RVhBTVBMRS1IQU5ETEU")
        );
        assert!(!page.posts[0].identity.is_external);
        assert!(page.posts[1].identity.is_external);
        assert_eq!(page.posts[0].title.as_deref(), Some("Example post 1"));
    }

    /// A list with nothing in it answers with an empty list where the page would be. So does a
    /// request with `fields`.
    #[test]
    fn test_a_page_of_a_list_sent_as_an_empty_list() {
        let page: ReaderListPostsResponse = serde_json::from_str("[]").expect("the page parses");
        assert!(page.posts.is_empty());
        assert!(page.unreadable_posts.is_empty());
        assert_eq!(page.missing_posts, 0);
        assert_eq!(page.next_page_handle, None);
    }

    #[test]
    fn test_a_page_of_a_list_keeps_its_posts_when_a_card_cannot_be_read() {
        let page: ReaderListPostsResponse = serde_json::from_str(
            &json!({
                "cards": [
                    { "type": "post", "date": 1795057248, "data": { "ID": 1, "site_ID": 10 } },
                    { "type": "post", "date": 1795057247, "data": false },
                    { "type": "post", "date": 1795057246, "data": null },
                    { "type": "post", "date": 1795057245 },
                    { "type": "post", "data": { "ID": 2, "site_ID": 10, "date": UNREADABLE_DATE } },
                    { "type": "recommended_blogs", "data": [{ "ID": 3 }] },
                    "not a card",
                    { "type": "post", "date": 1795057244, "data": { "ID": 4, "site_ID": 11 } },
                ],
                "next_page_handle": "abc",
            })
            .to_string(),
        )
        .expect("the page parses");

        assert_eq!(ids(&page.posts), [1, 4]);
        assert_eq!(page.missing_posts, 3);
        assert_eq!(page.next_page_handle.as_deref(), Some("abc"));

        assert_eq!(page.unreadable_posts.len(), 3);
        let by_id = |id| {
            page.unreadable_posts
                .iter()
                .find(|post| post.identity.as_ref().map(|identity| identity.id) == id)
        };
        let post = by_id(Some(2)).expect("the post whose date can't be read");
        assert!(
            post.error.starts_with("Invalid date format"),
            "{}",
            post.error
        );
        let errors: Vec<_> = page
            .unreadable_posts
            .iter()
            .filter(|post| post.identity.is_none())
            .map(|post| post.error.as_str())
            .collect();
        assert_eq!(errors.len(), 2);
        assert!(errors.contains(&"expected a `post` card, not a `recommended_blogs` card"));
    }

    fn card_kinds(cards: &[ReaderCard]) -> Vec<&str> {
        cards
            .iter()
            .map(|card| match card {
                ReaderCard::Post { .. } => "post",
                ReaderCard::RecommendedSites { .. } => "recommended sites",
                ReaderCard::NewSites { .. } => "new sites",
                ReaderCard::SuggestedTags { .. } => "suggested tags",
                ReaderCard::Unknown { card_type } => card_type,
            })
            .collect()
    }

    /// A recording of Discover on WordPress.com, asked for with `tags`, trimmed, with the sites,
    /// authors and post details replaced.
    #[test]
    fn test_deserialize_discover() {
        let json = include_str!("../../tests/wpcom/reader_posts/discover.json");
        let page: ReaderDiscoverResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(
            card_kinds(&page.cards),
            ["post", "recommended sites", "suggested tags"]
        );
        assert!(page.unreadable_cards.is_empty());
        assert_eq!(
            page.next_page_handle.as_deref(),
            Some("RVhBTVBMRS1IQU5ETEU")
        );
        assert_eq!(
            page.user_interests,
            [
                ReaderTagSlug("photography".to_string()),
                ReaderTagSlug("travel".to_string())
            ]
        );

        let ReaderCard::Post { post } = &page.cards[0] else {
            panic!("the first card is a post");
        };
        assert_eq!(post.identity.id, 1001);
        assert_eq!(post.title.as_deref(), Some("Example post 1"));
        assert!(post.railcar.is_some());

        let ReaderCard::RecommendedSites { sites } = &page.cards[1] else {
            panic!("the second card is recommended sites");
        };
        assert_eq!(sites.len(), 2);
        assert_eq!(sites[0].site_id, Some(WpComSiteId(6001)));
        assert_eq!(sites[0].feed_id, Some(ReaderFeedId(7001)));
        assert_eq!(sites[0].name.as_deref(), Some("Example Site 1"));
        assert_eq!(
            sites[0].url.as_deref(),
            Some("https://examplesite1.wordpress.com")
        );
        assert_eq!(
            sites[0].icon_url.as_deref(),
            Some("https://example.com/icon.png")
        );
        assert_eq!(sites[0].is_jetpack, Some(true));
        assert_eq!(sites[0].subscribers_count, Some(10));
        assert_eq!(sites[1].icon_url, None, "The API sent `null` for its icon");

        let ReaderCard::SuggestedTags { tags } = &page.cards[2] else {
            panic!("the third card is suggested tags");
        };
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].slug, ReaderTagSlug("photography".to_string()));
        assert_eq!(tags[0].title.as_deref(), Some("Photography"));
        assert_eq!(tags[0].score, Some(1258.0));
    }

    /// A recording of the first posts stream on WordPress.com, trimmed, with the sites, authors
    /// and post details replaced. The post's site is too new to have a feed.
    #[test]
    fn test_deserialize_first_posts() {
        let json = include_str!("../../tests/wpcom/reader_posts/first_posts.json");
        let page: ReaderFirstPostsResponse = serde_json::from_str(json).expect("the page parses");

        assert_eq!(card_kinds(&page.cards), ["post", "new sites"]);
        assert!(page.unreadable_cards.is_empty());
        assert_eq!(
            page.next_page_handle.as_deref(),
            Some("RVhBTVBMRS1IQU5ETEU")
        );

        let ReaderCard::Post { post } = &page.cards[0] else {
            panic!("the first card is a post");
        };
        assert_eq!(post.identity.site_id, Some(WpComSiteId(2001)));
        assert_eq!(post.identity.feed_id, None);

        let ReaderCard::NewSites { sites } = &page.cards[1] else {
            panic!("the second card is new sites");
        };
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].site_id, Some(WpComSiteId(6001)));
        assert_eq!(sites[0].subscribers_count, Some(10));
    }

    /// Discover's last page: a card of recommended sites, no posts, no handle and no interests.
    #[test]
    fn test_deserialize_the_last_page_of_discover() {
        let page: ReaderDiscoverResponse = serde_json::from_str(
            r#"{ "cards": [{ "type": "recommended_blogs", "data": [{ "ID": 1, "name": "A" }] }] }"#,
        )
        .expect("the page parses");

        assert_eq!(card_kinds(&page.cards), ["recommended sites"]);
        assert_eq!(page.next_page_handle, None);
        assert!(page.user_interests.is_empty());
    }

    /// What the first posts stream sends when asked for 16 cards or more: an empty card of new
    /// sites, and an empty string for the handle.
    #[test]
    fn test_deserialize_first_posts_asked_for_too_many() {
        let page: ReaderFirstPostsResponse = serde_json::from_str(
            r#"{"cards":[{"type":"new_sites","data":[]}],"next_page_handle":"","success":true,"lang":"en"}"#,
        )
        .expect("the page parses");

        assert_eq!(page.cards, [ReaderCard::NewSites { sites: vec![] }]);
        assert_eq!(page.next_page_handle, None);
    }

    #[test]
    fn test_a_card_of_an_unknown_kind_keeps_its_name() {
        let page: ReaderDiscoverResponse = serde_json::from_str(
            r#"{
                "cards": [
                    { "type": "post", "data": { "ID": 1, "site_ID": 10 } },
                    { "type": "poll_of_the_day", "data": { "question": "?" } },
                    { "type": "divider" },
                    { "type": "post", "data": { "ID": 2, "site_ID": 11 } }
                ],
                "next_page_handle": "abc"
            }"#,
        )
        .expect("the page parses");

        assert_eq!(
            card_kinds(&page.cards),
            ["post", "poll_of_the_day", "divider", "post"]
        );
        assert!(page.unreadable_cards.is_empty());
        assert_eq!(page.next_page_handle.as_deref(), Some("abc"));
    }

    #[test]
    fn test_a_page_of_cards_keeps_its_cards_when_one_cannot_be_read() {
        let page: ReaderDiscoverResponse = serde_json::from_str(
            &json!({
                "cards": [
                    { "type": "post", "data": { "ID": 1, "site_ID": 10 } },
                    { "type": "post", "data": { "ID": 2, "site_ID": 10, "date": UNREADABLE_DATE } },
                    { "type": "post", "data": false },
                    { "data": { "ID": 3 } },
                    "not a card",
                    { "type": "recommended_blogs", "data": { "ID": 4 } },
                    { "type": "post", "data": { "ID": 5, "site_ID": 11 } },
                ],
                "next_page_handle": "abc",
            })
            .to_string(),
        )
        .expect("the page parses");

        assert_eq!(card_kinds(&page.cards), ["post", "post"]);
        assert_eq!(page.next_page_handle.as_deref(), Some("abc"));
        assert_eq!(page.unreadable_cards.len(), 5);

        let of_kind = |kind: Option<&str>| -> Vec<&ReaderUnreadableCard> {
            page.unreadable_cards
                .iter()
                .filter(|card| card.card_type.as_deref() == kind)
                .collect()
        };
        let posts = of_kind(Some("post"));
        assert_eq!(posts.len(), 2);
        let dated = posts
            .iter()
            .find(|card| card.post.is_some())
            .expect("the post whose date can't be read");
        assert_eq!(dated.post.as_ref().map(|post| post.id), Some(2));
        assert!(
            dated.error.starts_with("Invalid date format"),
            "{}",
            dated.error
        );
        assert!(
            posts
                .iter()
                .any(|card| card.error == "the card has no post")
        );
        assert_eq!(of_kind(Some("recommended_blogs")).len(), 1);
        let nameless = of_kind(None);
        assert_eq!(nameless.len(), 2);
        assert!(
            nameless
                .iter()
                .any(|card| card.error == "the card has no `type`")
        );
    }

    /// A site or a tag that can't be read is left out of its card, and listed.
    #[test]
    fn test_a_card_keeps_its_sites_and_tags_when_one_cannot_be_read() {
        let page: ReaderFirstPostsResponse = serde_json::from_str(
            &json!({
                "cards": [
                    { "type": "new_sites", "data": [{ "ID": 1, "name": "One" }, "not a site", { "ID": 2 }] },
                    { "type": "interests_you_may_like", "data": [{ "title": "No slug" }, { "slug": "art" }] },
                ],
            })
            .to_string(),
        )
        .expect("the page parses");

        let ReaderCard::NewSites { sites } = &page.cards[0] else {
            panic!("the first card is new sites");
        };
        let ids: Vec<_> = sites.iter().map(|site| site.site_id).collect();
        assert_eq!(ids, [Some(WpComSiteId(1)), Some(WpComSiteId(2))]);
        let ReaderCard::SuggestedTags { tags } = &page.cards[1] else {
            panic!("the second card is suggested tags");
        };
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].slug, ReaderTagSlug("art".to_string()));

        let kinds: Vec<_> = page
            .unreadable_cards
            .iter()
            .map(|card| card.card_type.as_deref())
            .collect();
        assert_eq!(kinds, [Some("new_sites"), Some("interests_you_may_like")]);
    }

    #[test]
    fn test_card_site_fields_that_vary_in_shape() {
        let site: ReaderCardSite = serde_json::from_value(json!({
            "ID": "12",
            "feed_ID": null,
            "feed_URL": null,
            "name": "",
            "icon": [],
            "jetpack": 1,
            "subscribers_count": "7",
            "blog_id": null,
        }))
        .unwrap();
        assert_eq!(site.site_id, Some(WpComSiteId(12)));
        assert_eq!(site.feed_id, None);
        assert_eq!(site.feed_url, None);
        assert_eq!(site.name, None);
        assert_eq!(site.icon_url, None);
        assert_eq!(site.is_jetpack, Some(true));
        assert_eq!(site.subscribers_count, Some(7));

        let favicon: ReaderCardSite =
            serde_json::from_value(json!({ "ID": 1, "icon": { "ico": "https://a/favicon.ico" } }))
                .unwrap();
        assert_eq!(favicon.icon_url.as_deref(), Some("https://a/favicon.ico"));
    }

    /// A page of the posts of two tags is the shape of a page of one tag's posts.
    #[test]
    fn test_deserialize_the_posts_of_several_tags() {
        let page: ReaderTagPostsResponse = serde_json::from_value(json!({
            "success": true,
            "tags": ["photography", "travel"],
            "sort": "date",
            "lang": "en",
            "page": 1,
            "posts": [{ "ID": 1, "site_ID": 10 }, { "ID": 2, "site_ID": 11 }],
            "next_page_handle": "ZnJvbT0xMA==",
            "date_range": {
                "before": "2026-11-19T16:25:14+11:00",
                "after": "2026-11-19T00:01:00-05:00",
            },
        }))
        .expect("the page parses");

        assert_eq!(ids(&page.posts), [1, 2]);
        assert_eq!(page.next_page_handle.as_deref(), Some("ZnJvbT0xMA=="));
        assert!(page.date_range.is_some());
    }

    /// The API reads `order` in capitals only.
    #[test]
    fn test_stream_order_is_sent_in_capitals() {
        assert_eq!(ReaderStreamOrder::Asc.to_string(), "ASC");
        assert_eq!(ReaderStreamOrder::Desc.to_string(), "DESC");
        assert_eq!("ASC".parse(), Ok(ReaderStreamOrder::Asc));
    }

    #[test]
    fn test_tags_keep_the_order_the_api_sends() {
        // From a string: `json!` would sort the keys.
        let post: ReaderPost = serde_json::from_str(
            r#"{
                "ID": 9, "site_ID": 123, "is_external": false,
                "tags": {
                    "zebra": { "slug": "zebra", "name": "Zebra" },
                    "apple": { "slug": "apple", "name": "Apple" }
                }
            }"#,
        )
        .unwrap();
        let slugs: Vec<_> = post
            .tags
            .iter()
            .flatten()
            .map(|tag| tag.slug.as_str())
            .collect();
        assert_eq!(slugs, vec!["zebra", "apple"]);
    }

    #[rstest]
    #[case::empty_string(json!(""))]
    #[case::empty_object(json!({}))]
    #[case::empty_array(json!([]))]
    #[case::false_(json!(false))]
    #[case::null(json!(null))]
    fn test_no_tags(#[case] tags: serde_json::Value) {
        let post = parse(json!({ "ID": 9, "site_ID": 123, "is_external": false, "tags": tags }));
        assert_eq!(post.tags, Some(vec![]));
    }

    #[test]
    fn test_tags_left_out_are_not_known() {
        let post = parse(json!({ "ID": 9, "site_ID": 123, "is_external": false }));
        assert_eq!(post.tags, None, "Not the same as a post with no tags");
    }

    #[test]
    fn test_a_tag_without_a_slug_or_name_is_left_out() {
        let post = parse(json!({
            "ID": 9, "site_ID": 123, "is_external": false,
            "tags": {
                "news": { "slug": "news", "name": "News" },
                "no-name": { "slug": "no-name" },
                "no-slug": { "name": "No slug" },
            },
        }));
        let slugs: Vec<_> = post
            .tags
            .iter()
            .flatten()
            .map(|tag| tag.slug.as_str())
            .collect();
        assert_eq!(slugs, vec!["news"]);
    }

    #[test]
    fn test_a_non_empty_string_for_tags_is_an_error() {
        let result = serde_json::from_value::<ReaderPost>(
            json!({ "ID": 9, "site_ID": 123, "is_external": false, "tags": "news" }),
        );
        assert!(result.is_err());
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
        assert_eq!(post.tags, Some(vec![]));
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

    /// Real posts have a negative comment count. It isn't a count, so it reads as `None`, and the
    /// post is kept.
    #[test]
    fn test_comment_count_that_is_not_a_count() {
        let post = parse(json!({
            "ID": 9,
            "site_ID": 123,
            "discussion": { "comment_count": -8, "comments_open": true },
        }));
        assert_eq!(post.comment_count, None);
        assert_eq!(post.comments_open, Some(true));
    }

    /// Real posts have a featured image with a negative width and height. Neither is a size, so
    /// each reads as `None`, and the post and its image are kept.
    #[test]
    fn test_featured_media_with_sizes_that_are_not_sizes() {
        let post = parse(json!({
            "ID": 9,
            "site_ID": 123,
            "featured_media": {
                "uri": "https://example.com/image.jpg",
                "type": "image",
                "width": -340,
                "height": -334,
            },
        }));
        let media = post.featured_media.expect("the image is kept");
        assert_eq!(media.uri.as_deref(), Some("https://example.com/image.jpg"));
        assert_eq!(media.media_type.as_deref(), Some("image"));
        assert_eq!(media.width, None);
        assert_eq!(media.height, None);
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

    #[rstest::rstest]
    #[case::picked(json!({ "image": "https://editorial/img.jpg" }), Some("https://editorial/img.jpg"))]
    #[case::mshots(json!({ "image": MSHOTS }), None)]
    #[case::empty(json!({ "image": "" }), None)]
    #[case::no_image(json!({}), None)]
    #[case::not_sent(json!(null), None)]
    fn test_editorial_image_url(
        #[case] editorial: serde_json::Value,
        #[case] expected: Option<&str>,
    ) {
        let post = parse(json!({
            "ID": 1,
            "site_ID": 2,
            "is_external": false,
            "editorial": editorial,
            "featured_image": "https://featured/img.jpg",
        }));
        assert_eq!(post.editorial_image_url.as_deref(), expected);
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

    /// PHP sends a list whose keys aren't in sequence as an object.
    #[test]
    fn test_metadata_sent_as_an_object() {
        let post = parse(json!({
            "ID": 1,
            "site_ID": 2,
            "is_external": false,
            "metadata": {
                "0": { "id": "1", "key": "xpost_origin", "value": "123:456" },
                "2": { "id": "3", "key": "_xpost_original_permalink", "value": "https://example.com/p" },
            },
        }));
        assert_eq!(
            post.cross_post.and_then(|cross_post| cross_post.post_id),
            Some(456)
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

    /// With `fields`, a post arrives with only the fields asked for.
    #[test]
    fn test_a_page_of_posts_with_few_fields() {
        let page: ReaderPostsResponse = serde_json::from_value(json!({
            "found": 2,
            "posts": [
                { "ID": 1, "site_ID": 10, "date": "2026-11-17T09:00:00+00:00" },
                { "ID": 2, "site_ID": 10, "date": "2026-11-16T09:00:00+00:00" },
            ],
        }))
        .expect("the page parses");

        assert_eq!(ids(&page.posts), [1, 2]);
        assert!(page.unreadable_posts.is_empty());
        assert!(page.posts.iter().all(|post| post.date.is_some()));
        assert_eq!(page.posts[0].title, None);
    }

    /// Asked for `fields` that leave out `ID`, the API sends posts that can't be told apart.
    #[test]
    fn test_a_page_of_posts_without_their_ids() {
        let page: ReaderPostsResponse = serde_json::from_value(json!({
            "posts": [{ "title": "One" }, { "title": "Two" }],
        }))
        .expect("the page parses");

        assert!(page.posts.is_empty());
        assert_eq!(page.unreadable_posts.len(), 2);
    }

    /// With `fields`, a stream with no posts answers with an empty list where the page would be.
    #[test]
    fn test_a_page_sent_as_an_empty_list() {
        let page: ReaderPostsResponse = serde_json::from_str("[]").expect("the page parses");
        assert!(page.posts.is_empty());
        assert!(page.unreadable_posts.is_empty());
        assert_eq!(page.date_range, None);
    }

    /// A date WordPress.com sends for a post whose local and GMT dates don't agree.
    const UNREADABLE_DATE: &str = "2026-11-16T16:45:34+91:00";

    fn ids(posts: &[ReaderPost]) -> Vec<u64> {
        posts.iter().map(|post| post.identity.id).collect()
    }

    #[test]
    fn test_a_page_keeps_its_posts_when_one_cannot_be_read() {
        let page: ReaderPostsResponse = serde_json::from_value(json!({
            "found": 3,
            "posts": [
                { "ID": 1, "site_ID": 10, "date": "2026-11-17T09:00:00+00:00" },
                { "ID": 2, "site_ID": 10, "feed_ID": 20, "date": UNREADABLE_DATE },
                { "ID": 3, "site_ID": 10, "date": "2026-11-15T09:00:00+00:00" },
            ],
        }))
        .expect("the page parses");

        assert_eq!(ids(&page.posts), [1, 3]);
        assert_eq!(page.found, Some(3));
        assert_eq!(page.unreadable_posts.len(), 1);
        let unreadable = &page.unreadable_posts[0];
        let identity = unreadable.identity.as_ref().expect("its IDs were read");
        assert_eq!(identity.id, 2);
        assert_eq!(identity.site_id, Some(WpComSiteId(10)));
        assert_eq!(identity.feed_id, Some(ReaderFeedId(20)));
        assert!(
            unreadable
                .error
                .starts_with(&format!("Invalid date format: {UNREADABLE_DATE}")),
            "{}",
            unreadable.error
        );
    }

    #[test]
    fn test_a_page_of_a_tag_keeps_its_posts_when_one_cannot_be_read() {
        let page: ReaderTagPostsResponse = serde_json::from_value(json!({
            "posts": [
                { "ID": 1, "site_ID": 10 },
                { "ID": 2, "site_ID": 10, "modified": UNREADABLE_DATE },
            ],
            "next_page_handle": "ZnJvbT0z",
        }))
        .expect("the page parses");

        assert_eq!(ids(&page.posts), [1]);
        assert_eq!(page.next_page_handle.as_deref(), Some("ZnJvbT0z"));
        let unreadable: Vec<_> = page
            .unreadable_posts
            .iter()
            .map(|post| post.identity.as_ref().map(|identity| identity.id))
            .collect();
        assert_eq!(unreadable, [Some(2)]);
    }

    /// A post has to have an ID, so one without is unreadable, and there's no saying which it was.
    #[test]
    fn test_an_unreadable_post_whose_ids_cannot_be_read() {
        let page: ReaderPostsResponse = serde_json::from_value(json!({
            "posts": [{ "site_ID": 10, "title": "No ID" }, "not a post", { "ID": 3 }],
        }))
        .expect("the page parses");

        assert_eq!(ids(&page.posts), [3]);
        let identities: Vec<_> = page
            .unreadable_posts
            .iter()
            .map(|post| post.identity.clone())
            .collect();
        assert_eq!(identities, [None, None]);
    }

    #[rstest]
    #[case(json!({ "posts": [] }))]
    #[case(json!({ "posts": null }))]
    #[case(json!({}))]
    fn test_a_page_without_posts(#[case] page: serde_json::Value) {
        let page: ReaderPostsResponse = serde_json::from_value(page).expect("the page parses");
        assert!(page.posts.is_empty());
        assert!(page.unreadable_posts.is_empty());
    }

    /// Only a post that can't be read is passed over. A page that isn't a page is still an error.
    #[rstest]
    #[case(json!({ "posts": "none" }))]
    #[case(json!({ "posts": { "ID": 1 } }))]
    #[case(json!({ "posts": [], "found": [] }))]
    fn test_a_page_that_cannot_be_read_is_an_error(#[case] page: serde_json::Value) {
        assert!(serde_json::from_value::<ReaderPostsResponse>(page.clone()).is_err());
        assert!(serde_json::from_str::<ReaderPostsResponse>(&page.to_string()).is_err());
    }

    /// The oldest post's date is the page's `date_range.after`, so an unreadable one is in both.
    #[test]
    fn test_a_date_range_with_a_date_that_cannot_be_read() {
        let page: ReaderPostsResponse = serde_json::from_value(json!({
            "date_range": { "before": "2026-11-17T09:00:00+00:00", "after": UNREADABLE_DATE },
            "posts": [
                { "ID": 1, "date": "2026-11-17T09:00:00+00:00" },
                { "ID": 2, "date": UNREADABLE_DATE },
            ],
        }))
        .expect("the page parses");

        assert_eq!(ids(&page.posts), [1]);
        assert_eq!(page.unreadable_posts.len(), 1);
        let range = page.date_range.expect("the page has a range");
        assert_eq!(range.before, page.posts[0].date);
        assert_eq!(range.unreadable_before, None);
        assert_eq!(range.after, None);
        assert_eq!(range.unreadable_after.as_deref(), Some(UNREADABLE_DATE));
    }

    #[rstest]
    #[case(json!(null), None, None)]
    #[case(json!(""), None, None)]
    #[case(json!("0000-00-00 00:00:00"), None, None)]
    #[case(json!("2026-11-17T09:00:00+00:00"), Some("2026-11-17T09:00:00+00:00"), None)]
    #[case(json!("yesterday"), None, Some("yesterday"))]
    #[case(json!({ "date": 1 }), None, Some(r#"{"date":1}"#))]
    fn test_date_range_bound(
        #[case] sent: serde_json::Value,
        #[case] date: Option<&str>,
        #[case] unreadable: Option<&str>,
    ) {
        let range: ReaderDateRange =
            serde_json::from_value(json!({ "after": sent })).expect("the range parses");
        assert_eq!(range.after, date.map(|date| date.parse().unwrap()));
        assert_eq!(range.unreadable_after.as_deref(), unreadable);
        assert_eq!((range.before, range.unreadable_before), (None, None));
    }
}
