//! One Reader post at a time: a site's post, a feed's post, a post found by its slug, and the posts
//! related to a post.
//!
//! A site's post and a feed's post arrive in the shape the streams send, so each is a
//! [`ReaderPost`]. Which fields they carry differs a little from a stream's post:
//!
//! - A site's post has its `feed_ID` but never its `feed_item_ID`, which a stream sends for a post
//!   recent enough to still be in the feed. Don't take a missing
//!   [`ReaderPostSaveIdentity::feed_item_id`](super::reader_saved::ReaderPostSaveIdentity::feed_item_id)
//!   from this request to mean the post has none.
//! - A feed's post has both. When the feed belongs to a WordPress.com or Jetpack site, it's the
//!   site's post: its `ID` is the post ID, and it isn't external. It also has
//!   [`ReaderPost::word_count`], which no stream sends, and [`ReaderPost::is_jetpack`], which a
//!   site's post leaves out. An RSS post asked for on its own has nothing about comment
//!   subscriptions, which the same post in a stream has.
//!
//! Apart from those, a post asked for on its own has the fields the same post has in a stream, so
//! the two can be kept as one. Its content isn't the same text every time, though: a post with an
//! image block arrives with a new `data-wp-key` on each request.
//!
//! A post found by its slug comes from the general posts endpoint, which sends a different shape.
//! See [`ReaderPostBySlug`].

use crate::{
    date::WpGmtDateTime,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_com::{
        WpComSiteId,
        reader_posts::{ReaderPost, ReaderPostAuthor, ReaderPostTag, ReaderUnreadablePost},
        reader_saved::{ReaderPostSaveIdentity, deserialize_optional_text},
    },
};
use serde::{Deserialize, Serialize};
use std::fmt;
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::LenientVec;

/// Query parameters for one post of a site or a feed.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct ReaderPostParams {
    /// Extra data to include, comma-separated. For a site's post, `site` sends the post's site,
    /// which has its [`ReaderPost::site_locale`], and `likes` its likes. For a feed's post, `feed`
    /// sends the feed, which has an RSS feed's image for [`ReaderPost::site_icon_url`].
    ///
    /// Signed out, the site arrives without its locale.
    #[uniffi(default = None)]
    pub meta: Option<String>,
    /// The fields of the post to return, comma-separated, such as `ID,site_ID,date`. Without it
    /// the post arrives with every field.
    ///
    /// `ID` has to be among them: a post without one can't be read, and the request fails.
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

uniffi::custom_newtype!(ReaderPostSlug, String);
/// A post's slug, the last part of its address: `hello-world` in
/// `https://example.com/2026/11/19/hello-world/`.
///
/// As text it's the path segment the API takes, `slug:hello-world`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReaderPostSlug(pub String);

impl fmt::Display for ReaderPostSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slug:{}", self.0)
    }
}

/// Query parameters for a post found by its slug.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct ReaderPostBySlugParams {
    /// The fields of the post to return, comma-separated. `ID,site_ID` is enough to then fetch the
    /// post as a [`ReaderPost`], and leaves out the post's content.
    ///
    /// `ID` and `site_ID` have to be among them, or the request fails.
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

/// A post found by its slug.
///
/// It comes from the general posts endpoint rather than a Reader one, so it isn't a
/// [`ReaderPost`]. It has no feed ID, no `is_external`, no site name, address or icon, no
/// `featured_media`, no seen state and nothing about comment subscriptions. Stored as a Reader
/// post, it would overwrite all of those with nothing.
///
/// Use it to find a post's IDs from its address, then fetch the post itself with the IDs, which
/// returns a [`ReaderPost`]. The fields here are enough to show something in the meantime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(try_from = "RawReaderPostBySlug")]
pub struct ReaderPostBySlug {
    pub post_id: PostId,
    pub site_id: WpComSiteId,
    #[uniffi(default = None)]
    pub global_id: Option<String>,
    #[uniffi(default = None)]
    pub slug: Option<String>,
    #[uniffi(default = None)]
    pub url: Option<String>,
    #[uniffi(default = None)]
    pub short_url: Option<String>,
    /// `publish` for a post anyone can read. Someone who can edit the site also gets its other
    /// posts, such as a `draft`, a `private` or a `trash` one.
    #[uniffi(default = None)]
    pub status: Option<String>,
    /// `post`, or `page` for a page, which the API finds by its slug too.
    #[uniffi(default = None)]
    pub post_type: Option<String>,
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
    #[uniffi(default = None)]
    pub date: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub modified: Option<WpGmtDateTime>,
    #[uniffi(default = None)]
    pub author: Option<ReaderPostAuthor>,
    #[uniffi(default = None)]
    pub featured_image: Option<String>,
    /// In the order the API sends them. `None` when the response left out `tags`, and empty when
    /// the post has none.
    #[uniffi(default = None)]
    pub tags: Option<Vec<ReaderPostTag>>,
    #[uniffi(default = None)]
    pub like_count: Option<u64>,
    #[uniffi(default = None)]
    pub i_like: Option<bool>,
    #[uniffi(default = None)]
    pub likes_enabled: Option<bool>,
    #[uniffi(default = None)]
    pub is_following: Option<bool>,
    #[uniffi(default = None)]
    pub comment_count: Option<u64>,
    #[uniffi(default = None)]
    pub comments_open: Option<bool>,
}

/// The fields a post found by its slug shares with a Reader post are read the way a Reader post's
/// are, so they tolerate the same variations. The rest of what [`ReaderPost`] would hold is left
/// behind.
#[derive(Deserialize)]
struct RawReaderPostBySlug {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    slug: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    status: Option<String>,
    #[serde(
        rename = "type",
        default,
        deserialize_with = "deserialize_optional_text"
    )]
    post_type: Option<String>,
    #[serde(flatten)]
    post: ReaderPost,
}

impl TryFrom<RawReaderPostBySlug> for ReaderPostBySlug {
    type Error = String;

    fn try_from(raw: RawReaderPostBySlug) -> Result<Self, Self::Error> {
        let post = raw.post;
        let ReaderPostSaveIdentity {
            id, site_id, url, ..
        } = post.identity;
        let post_id = i64::try_from(id)
            .map(PostId)
            .map_err(|_| format!("`ID` {id} is out of range"))?;
        let site_id = site_id.ok_or("a post found by its slug needs a `site_ID`")?;
        Ok(Self {
            post_id,
            site_id,
            global_id: post.global_id,
            slug: raw.slug,
            url,
            short_url: post.short_url,
            status: raw.status,
            post_type: raw.post_type,
            title: post.title,
            excerpt: post.excerpt,
            content: post.content,
            date: post.date,
            modified: post.modified,
            author: post.author,
            featured_image: post.featured_image,
            tags: post.tags,
            like_count: post.like_count,
            i_like: post.i_like,
            likes_enabled: post.likes_enabled,
            is_following: post.is_following,
            comment_count: post.comment_count,
            comments_open: post.comments_open,
        })
    }
}

/// Query parameters for the posts related to a post.
///
/// Ask for some with [`Self::size_local`], [`Self::size_global`] or both: without either, the API
/// returns none. There's one page.
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)] // There's one page.
pub struct ReaderRelatedPostsParams {
    /// How many posts from other sites to return: 20 at most, or the request fails.
    #[uniffi(default = None)]
    pub size_global: Option<u32>,
    /// How many posts from the same site to return: 20 at most, or the request fails.
    #[uniffi(default = None)]
    pub size_local: Option<u32>,
    /// The language to look for posts in. It showed no effect when this was written: posts from
    /// other sites came back in the post's own language whatever was asked for.
    #[uniffi(default = None)]
    pub lang: Option<String>,
    /// The name of the algorithm to pick every post with. The response names it back. A name the
    /// API doesn't know still returns posts.
    #[uniffi(default = None)]
    pub algorithm: Option<String>,
    /// The name of the algorithm to pick the same site's posts with.
    #[uniffi(default = None)]
    pub algorithm_local: Option<String>,
    /// The name of the algorithm to pick other sites' posts with.
    #[uniffi(default = None)]
    pub algorithm_global: Option<String>,
    /// Whether to shuffle the same site's posts before taking [`Self::size_local`] of them. The
    /// API's default is not to.
    #[uniffi(default = None)]
    pub shuffle_posts_local: Option<bool>,
    /// Whether to shuffle other sites' posts before taking [`Self::size_global`] of them. The
    /// API's default is to. Their order differs from one request to the next either way.
    #[uniffi(default = None)]
    pub shuffle_posts_global: Option<bool>,
    /// Extra data to include, comma-separated: `site` sends each post's site, which has its
    /// [`ReaderPost::site_locale`].
    #[uniffi(default = None)]
    pub meta: Option<String>,
    /// The fields of each post to return, comma-separated, such as `ID,site_ID,railcar`. `ID` has
    /// to be among them. A post without one can't be read, and is listed in
    /// [`ReaderRelatedPostsResponse::unreadable_posts`].
    #[uniffi(default = None)]
    pub fields: Option<String>,
}

/// The posts related to a post: the same site's first, then other sites'.
///
/// Each is a post as a stream sends it, with a [`ReaderPost::railcar`] that says how it was
/// picked. None has a `feed_item_ID` or a seen state. Each has a [`ReaderPost::tagged_on`], which
/// here is the time of the request rather than when the post was tagged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
#[serde(from = "RawReaderRelatedPostsResponse")]
pub struct ReaderRelatedPostsResponse {
    /// The related posts, without any in [`Self::unreadable_posts`]. Empty when none were asked
    /// for, and when the API finds none.
    pub posts: Vec<ReaderPost>,
    /// The related posts that couldn't be read. Almost always empty.
    #[uniffi(default = [])]
    pub unreadable_posts: Vec<ReaderUnreadablePost>,
    /// The algorithm that picked the same site's posts. `None` when none were asked for.
    #[uniffi(default = None)]
    pub algorithm_local: Option<String>,
    /// The algorithm that picked other sites' posts. `None` when none were asked for.
    #[uniffi(default = None)]
    pub algorithm_global: Option<String>,
}

#[derive(Deserialize)]
struct RawReaderRelatedPostsResponse {
    #[serde(default)]
    posts: Option<LenientVec<ReaderPost>>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    algorithm_local: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    algorithm_global: Option<String>,
}

impl From<RawReaderRelatedPostsResponse> for ReaderRelatedPostsResponse {
    fn from(raw: RawReaderRelatedPostsResponse) -> Self {
        let posts = raw.posts.unwrap_or_default();
        let unreadable_posts = posts
            .invalid
            .into_iter()
            .map(|post| ReaderUnreadablePost {
                identity: serde_json::from_str(post.json.get()).ok(),
                error: post.error.to_string(),
            })
            .collect();
        Self {
            posts: posts.valid,
            unreadable_posts,
            algorithm_local: raw.algorithm_local,
            algorithm_global: raw.algorithm_global,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wp_com::reader_saved::{
        ReaderFeedId, ReaderFeedItemId, ReaderSavedKey, reader_saved_key_for_post,
    };
    use serde_json::json;

    /// A recording of one post of a WordPress.com site, trimmed, with the site, author and post
    /// details replaced.
    #[test]
    fn test_deserialize_site_post() {
        let json = include_str!("../../tests/wpcom/reader_post/site_post.json");
        let post: ReaderPost = serde_json::from_str(json).expect("the post parses");

        assert_eq!(post.identity.id, 1001);
        assert_eq!(post.identity.site_id, Some(WpComSiteId(2001)));
        assert_eq!(post.identity.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(
            post.identity.feed_item_id, None,
            "A site's post never has its feed item ID"
        );
        assert!(!post.identity.is_external);
        assert_eq!(
            reader_saved_key_for_post(post.identity.clone()),
            Some(ReaderSavedKey::Post {
                blog_id: WpComSiteId(2001),
                post_id: PostId(1001),
            })
        );
        assert_eq!(post.title.as_deref(), Some("Example post"));
        assert_eq!(post.site_name.as_deref(), Some("Example Site"));
        assert!(post.site_icon_url.is_some());
        assert_eq!(post.is_seen, Some(false));
        assert_eq!(post.like_count, Some(45));
        assert_eq!(post.comments_open, Some(true));
        assert_eq!(post.tags.as_ref().map(Vec::len), Some(1));
        assert_eq!(post.word_count, None, "Only a feed's post has it");
    }

    /// A recording of one post of an RSS feed. Its `site_ID` is a string that repeats the feed ID,
    /// `i_like` is `0`, and `metadata` is `false`.
    #[test]
    fn test_deserialize_feed_post() {
        let json = include_str!("../../tests/wpcom/reader_post/feed_post.json");
        let post: ReaderPost = serde_json::from_str(json).expect("the post parses");

        assert!(post.identity.is_external);
        assert_eq!(post.identity.id, 4001);
        assert_eq!(post.identity.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(post.identity.feed_item_id, Some(ReaderFeedItemId(4001)));
        assert_eq!(post.identity.site_id, Some(WpComSiteId(3001)));
        assert_eq!(
            reader_saved_key_for_post(post.identity.clone()),
            Some(ReaderSavedKey::FeedItem {
                feed_item_id: ReaderFeedItemId(4001),
            })
        );
        assert_eq!(post.i_like, Some(false));
        assert_eq!(post.tags, Some(vec![]));
        assert_eq!(post.featured_media, None);
        assert_eq!(post.cross_post, None);
        assert_eq!(post.is_jetpack, Some(false));
        assert_eq!(post.word_count, Some(0));
    }

    /// A recording of one post of a feed that belongs to a WordPress.com site. It's the site's
    /// post, with its feed item ID, and `like_count` is a string.
    #[test]
    fn test_deserialize_feed_post_of_a_site() {
        let json = include_str!("../../tests/wpcom/reader_post/feed_post_of_a_site.json");
        let post: ReaderPost = serde_json::from_str(json).expect("the post parses");

        assert!(!post.identity.is_external);
        assert_eq!(post.identity.id, 1001, "The post ID, not the feed item ID");
        assert_eq!(post.identity.site_id, Some(WpComSiteId(2001)));
        assert_eq!(post.identity.feed_id, Some(ReaderFeedId(3001)));
        assert_eq!(post.identity.feed_item_id, Some(ReaderFeedItemId(4001)));
        assert_eq!(post.like_count, Some(45));
        assert_eq!(post.word_count, Some(921));
        assert_eq!(post.is_jetpack, Some(false));
    }

    /// A recording of a post found by its slug, trimmed, with its details replaced.
    #[test]
    fn test_deserialize_post_by_slug() {
        let json = include_str!("../../tests/wpcom/reader_post/post_by_slug.json");
        let post: ReaderPostBySlug = serde_json::from_str(json).expect("the post parses");

        assert_eq!(post.post_id, PostId(1001));
        assert_eq!(post.site_id, WpComSiteId(2001));
        assert_eq!(post.slug.as_deref(), Some("example-post"));
        assert_eq!(post.status.as_deref(), Some("publish"));
        assert_eq!(post.post_type.as_deref(), Some("post"));
        assert_eq!(post.title.as_deref(), Some("Example post"));
        assert_eq!(
            post.url.as_deref(),
            Some("https://example.wordpress.com/2026/11/19/example-post/")
        );
        assert!(post.content.is_some() && post.date.is_some());
        assert_eq!(
            post.author.as_ref().and_then(|author| author.id),
            Some(5001)
        );
        assert_eq!(post.tags.as_ref().map(Vec::len), Some(1));
        assert_eq!(post.like_count, Some(45));
        assert_eq!(post.comment_count, Some(0));
        assert_eq!(post.comments_open, Some(true));
    }

    /// Asked for `fields=ID,site_ID`, the API sends only those.
    #[test]
    fn test_a_post_by_slug_with_only_its_ids() {
        let post: ReaderPostBySlug =
            serde_json::from_value(json!({ "ID": 1001, "site_ID": 2001 })).expect("it parses");
        assert_eq!(post.post_id, PostId(1001));
        assert_eq!(post.site_id, WpComSiteId(2001));
        assert_eq!(post.title, None);
        assert_eq!(post.tags, None);
    }

    #[rstest::rstest]
    #[case::no_post_id(json!({ "site_ID": 2001, "title": "No ID" }))]
    #[case::no_site_id(json!({ "ID": 1001, "title": "No site" }))]
    fn test_a_post_by_slug_without_its_ids_is_an_error(#[case] post: serde_json::Value) {
        assert!(serde_json::from_value::<ReaderPostBySlug>(post).is_err());
    }

    #[test]
    fn test_post_slug_is_the_path_segment() {
        assert_eq!(
            ReaderPostSlug("hello-world".to_string()).to_string(),
            "slug:hello-world"
        );
    }

    /// A recording of related posts, one from the same site and one from another, trimmed, with
    /// their details replaced.
    #[test]
    fn test_deserialize_related_posts() {
        let json = include_str!("../../tests/wpcom/reader_post/related_posts.json");
        let related: ReaderRelatedPostsResponse =
            serde_json::from_str(json).expect("the response parses");

        assert_eq!(
            related.algorithm_local.as_deref(),
            Some("read:related:posts:local/7")
        );
        assert!(related.algorithm_global.is_some());
        assert!(related.unreadable_posts.is_empty());
        let sites: Vec<_> = related
            .posts
            .iter()
            .map(|post| post.identity.site_id)
            .collect();
        assert_eq!(sites, [Some(WpComSiteId(2001)), Some(WpComSiteId(2002))]);
        for post in &related.posts {
            assert!(post.railcar.is_some());
            assert_eq!(post.identity.feed_item_id, None);
            assert_eq!(post.is_seen, None);
        }
    }

    /// What the API sends when no posts were asked for, and when it finds none.
    #[test]
    fn test_no_related_posts() {
        let related: ReaderRelatedPostsResponse =
            serde_json::from_value(json!({ "posts": [] })).expect("the response parses");
        assert!(related.posts.is_empty());
        assert_eq!(related.algorithm_local, None);
        assert_eq!(related.algorithm_global, None);
    }

    /// Asked for one kind only, the API names one algorithm.
    #[test]
    fn test_related_posts_from_other_sites_only() {
        let related: ReaderRelatedPostsResponse = serde_json::from_value(json!({
            "algorithm_global": "read:related:posts:global/2",
            "posts": [{ "ID": 1, "site_ID": 10 }],
        }))
        .expect("the response parses");
        assert_eq!(related.posts.len(), 1);
        assert_eq!(related.algorithm_local, None);
        assert_eq!(
            related.algorithm_global.as_deref(),
            Some("read:related:posts:global/2")
        );
    }

    #[test]
    fn test_related_posts_keep_the_rest_when_one_cannot_be_read() {
        let related: ReaderRelatedPostsResponse = serde_json::from_value(json!({
            "posts": [
                { "ID": 1, "site_ID": 10 },
                { "ID": 2, "site_ID": 11, "date": "2026-11-16T16:45:34+91:00" },
                { "ID": 3, "site_ID": 12 },
            ],
        }))
        .expect("the response parses");

        let ids: Vec<_> = related.posts.iter().map(|post| post.identity.id).collect();
        assert_eq!(ids, [1, 3]);
        assert_eq!(related.unreadable_posts.len(), 1);
        let identity = related.unreadable_posts[0]
            .identity
            .as_ref()
            .expect("its IDs were read");
        assert_eq!(identity.id, 2);
        assert_eq!(identity.site_id, Some(WpComSiteId(11)));
    }
}
