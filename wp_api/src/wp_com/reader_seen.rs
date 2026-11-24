//! Seen status: whether the user has seen a post in a stream of followed sites.
//!
//! A post supports seen status when the stream sent it with `is_seen`, which the Following stream
//! does. A post from a WordPress.com or Jetpack site is marked by its site and post IDs, and an RSS
//! post by its feed and feed item IDs: use the pair that matches
//! [`ReaderPostSaveIdentity::is_feed_item`](crate::wp_com::reader_saved::ReaderPostSaveIdentity::is_feed_item).
//! A feed item with no post ID (`0`) comes with `is_external: false`, and is marked by its feed.

use crate::{
    posts::PostId,
    wp_com::{
        WpComSiteId,
        reader_saved::{ReaderFeedId, ReaderFeedItemId},
    },
};
use serde::{Deserialize, Serialize};

/// The app marking a post, recorded with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum ReaderSeenSource {
    #[serde(rename = "reader-android")]
    Android,
    #[serde(rename = "reader-ios")]
    Ios,
    #[serde(rename = "reader-web")]
    Web,
}

/// Request body to mark posts of a WordPress.com or Jetpack site as seen, or as unseen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderSeenSitePostsParams {
    /// The site the posts are on.
    pub blog_id: WpComSiteId,
    /// At least one. The API refuses an empty list with `rest_invalid_param`.
    pub post_ids: Vec<PostId>,
    /// The app marking the posts, which is recorded with the mark.
    pub source: ReaderSeenSource,
}

/// Request body to mark posts of an RSS feed as seen, or as unseen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, uniffi::Record)]
pub struct ReaderSeenFeedItemsParams {
    /// The feed the posts are in.
    pub feed_id: ReaderFeedId,
    /// At least one.
    pub feed_item_ids: Vec<ReaderFeedItemId>,
    /// The app marking the posts, which is recorded with the mark.
    pub source: ReaderSeenSource,
}

/// The answer to marking posts as seen or unseen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderSeenResponse {
    /// Whether the posts were marked. Marking a post that's already in that state is `true` too.
    pub status: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_the_body_for_a_sites_posts() {
        let params = ReaderSeenSitePostsParams {
            blog_id: WpComSiteId(123),
            post_ids: vec![PostId(4), PostId(5)],
            source: ReaderSeenSource::Ios,
        };
        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            json!({ "blog_id": 123, "post_ids": [4, 5], "source": "reader-ios" })
        );
    }

    #[test]
    fn test_the_body_for_a_feeds_items() {
        let params = ReaderSeenFeedItemsParams {
            feed_id: ReaderFeedId(123),
            feed_item_ids: vec![ReaderFeedItemId(4)],
            source: ReaderSeenSource::Android,
        };
        assert_eq!(
            serde_json::to_value(&params).unwrap(),
            json!({ "feed_id": 123, "feed_item_ids": [4], "source": "reader-android" })
        );
    }

    #[test]
    fn test_the_answer() {
        // Marking as seen sends a message as well, and marking as unseen doesn't.
        let seen: ReaderSeenResponse = serde_json::from_value(
            json!({ "status": true, "message": "Feed items marked as seen." }),
        )
        .unwrap();
        let unseen: ReaderSeenResponse = serde_json::from_value(json!({ "status": true })).unwrap();
        assert!(seen.status && unseen.status);
    }
}
