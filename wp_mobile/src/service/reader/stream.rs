//! The Reader's streams: which endpoint each one reads, how it pages, and what orders its posts.

use wp_api::date::WpGmtDateTime;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_posts::{
    ReaderPost, ReaderStreamParams, ReaderTagPostsParams, ReaderTagPostsSort, ReaderTagSlug,
    ReaderUnreadablePost,
};
use wp_api::wp_com::reader_saved::ReaderFeedId;

/// How many posts a page of a stream ordered by date asks for.
const DATED_PAGE_SIZE: u32 = 20;

/// What a stream asks for with each post: the post's site, and an RSS post's feed. They carry the
/// site's or feed's icon and the site's language, which a post doesn't have without them.
const POST_META: &str = "site,feed";

/// A stream of posts the Reader shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderStream {
    /// Posts from the sites and feeds the user follows. Needs an account.
    Following,
    /// A WordPress.com or Jetpack site's posts.
    Site { site_id: WpComSiteId },
    /// An RSS feed's posts.
    Feed { feed_id: ReaderFeedId },
    /// A tag's posts, newest first.
    Tag { slug: ReaderTagSlug },
}

/// What orders a stream's posts, which decides how the cache keeps them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderStreamOrder {
    /// By a date each post has. A page is asked for by date, so a refresh merges into what the
    /// cache has, and a page can be loaded anywhere in the stream: at its end, or into a gap.
    Date,
    /// The order the server sent, paged with a handle the previous page gave. Later pages follow
    /// from the first, so a refresh replaces what the cache has.
    Server,
}

impl ReaderStream {
    /// The stream's key in the cache. It never changes for a stream, since the cache finds the
    /// stream's posts by it.
    pub(crate) fn key(&self) -> String {
        match self {
            Self::Following => "following".to_string(),
            Self::Site { site_id } => format!("site:{}", site_id.0),
            Self::Feed { feed_id } => format!("feed:{}", feed_id.0),
            Self::Tag { slug } => format!("tag:{}", slug.0),
        }
    }

    pub(crate) fn needs_account(&self) -> bool {
        match self {
            Self::Following => true,
            Self::Site { .. } | Self::Feed { .. } | Self::Tag { .. } => false,
        }
    }

    pub(crate) fn order(&self) -> ReaderStreamOrder {
        match self {
            Self::Following | Self::Site { .. } | Self::Feed { .. } => ReaderStreamOrder::Date,
            Self::Tag { .. } => ReaderStreamOrder::Server,
        }
    }

    /// Fetches a page: the first when `cursor` is `None`.
    pub(crate) async fn fetch(
        &self,
        client: &WpComApiClient,
        cursor: Option<&StreamCursor>,
    ) -> Result<StreamPage, WpApiError> {
        let posts = client.reader_posts();
        match self {
            Self::Following | Self::Site { .. } | Self::Feed { .. } => {
                let params = ReaderStreamParams {
                    number: Some(DATED_PAGE_SIZE),
                    before: match cursor {
                        Some(StreamCursor::Before(date)) => Some(*date),
                        _ => None,
                    },
                    meta: Some(POST_META.to_string()),
                    ..Default::default()
                };
                let page = match self {
                    Self::Following => posts.following(&params).await?.data,
                    Self::Site { site_id } => posts.site_posts(site_id, &params).await?.data,
                    Self::Feed { feed_id } => posts.feed_posts(feed_id, &params).await?.data,
                    Self::Tag { .. } => unreachable!("a tag's stream is fetched below"),
                };
                Ok(StreamPage {
                    posts: page.posts.into_iter().map(dated_by_its_date).collect(),
                    unreadable: page.unreadable_posts,
                    next_page_handle: None,
                })
            }
            Self::Tag { slug } => {
                let params = ReaderTagPostsParams {
                    page_handle: match cursor {
                        Some(StreamCursor::Handle(handle)) => Some(handle.clone()),
                        _ => None,
                    },
                    sort: Some(ReaderTagPostsSort::Date),
                    ..Default::default()
                };
                let page = posts.tag_posts(slug, &params).await?.data;
                Ok(StreamPage {
                    posts: page.posts.into_iter().map(dated_by_its_date).collect(),
                    unreadable: page.unreadable_posts,
                    next_page_handle: page.next_page_handle,
                })
            }
        }
    }
}

/// Whether a stream needs a WordPress.com account. Without one, it fails with
/// [`super::ReaderError::NeedsAccount`].
#[uniffi::export]
pub fn reader_stream_needs_account(stream: ReaderStream) -> bool {
    stream.needs_account()
}

/// What orders a stream's posts.
#[uniffi::export]
pub fn reader_stream_order(stream: ReaderStream) -> ReaderStreamOrder {
    stream.order()
}

/// Where a page starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamCursor {
    /// The posts older than a date, for a stream ordered by date.
    Before(WpGmtDateTime),
    /// The page a previous page's handle leads to.
    Handle(String),
}

/// A page of a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamPage {
    /// In the order the server sent.
    pub(crate) posts: Vec<StreamPost>,
    pub(crate) unreadable: Vec<ReaderUnreadablePost>,
    /// The handle of the next page of a stream that pages with one, or `None` on its last page.
    pub(crate) next_page_handle: Option<String>,
}

/// A post on a page, with the date its stream orders it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamPost {
    pub(crate) post: ReaderPost,
    /// `None` when the post doesn't have the date. A stream ordered by date can't place it then.
    pub(crate) sort_date: Option<WpGmtDateTime>,
}

fn dated_by_its_date(post: ReaderPost) -> StreamPost {
    StreamPost {
        sort_date: post.date,
        post,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(ReaderStream::Following, "following", true, ReaderStreamOrder::Date)]
    #[case(ReaderStream::Site { site_id: WpComSiteId(3584907) }, "site:3584907", false, ReaderStreamOrder::Date)]
    #[case(ReaderStream::Feed { feed_id: ReaderFeedId(8857199) }, "feed:8857199", false, ReaderStreamOrder::Date)]
    #[case(ReaderStream::Tag { slug: ReaderTagSlug("photography".to_string()) }, "tag:photography", false, ReaderStreamOrder::Server)]
    fn test_what_each_stream_is(
        #[case] stream: ReaderStream,
        #[case] key: &str,
        #[case] needs_account: bool,
        #[case] order: ReaderStreamOrder,
    ) {
        assert_eq!(stream.key(), key);
        assert_eq!(stream.needs_account(), needs_account);
        assert_eq!(stream.order(), order);
    }
}
