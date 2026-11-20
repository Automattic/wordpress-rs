//! The Reader's streams: which endpoint each one reads, how it pages, and what orders its posts.

use serde::{Deserialize, Serialize};
use wp_api::date::WpGmtDateTime;
use wp_api::posts::PostId;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_post::ReaderRelatedPostsParams;
use wp_api::wp_com::reader_posts::{
    ReaderCard, ReaderCardSite, ReaderCardTag, ReaderDiscoverParams, ReaderFirstPostsParams,
    ReaderFreshlyPressedParams, ReaderLikedPostsParams, ReaderListOwner, ReaderListPostsParams,
    ReaderListSlug, ReaderOrganizationPostsParams, ReaderPost, ReaderPostsResponse,
    ReaderSearchPostsParams, ReaderSearchSort, ReaderStreamParams, ReaderTagPostsParams,
    ReaderTagPostsResponse, ReaderTagPostsSort, ReaderTagSlug, ReaderTagsPostsParams,
    ReaderUnreadableCard, ReaderUnreadablePost,
};
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderPostSaveIdentity};
use wp_api::wp_com::reader_tags::{ReaderMenuItem, ReaderMenuItemKind};

use super::language::ReaderLanguage;

/// How many posts a page of a stream ordered by date asks for.
const DATED_PAGE_SIZE: u32 = 20;

/// How many posts a page of search results asks for, which is the most the API gives.
const SEARCH_PAGE_SIZE: u32 = 20;

/// How many related posts are asked for from the post's own site, and how many from other sites.
/// The API returns none of a kind that isn't asked for. Two of each, as both WordPress apps ask
/// for (Android's `NUM_RELATED_POSTS_TO_REQUEST` and iOS's `fetchRelatedPosts` default, on their
/// trunks of 2026-11-17).
const RELATED_POSTS_OF_EACH_KIND: u32 = 2;

/// What a stream asks for with each post: the post's site, and an RSS post's feed. They carry the
/// site's or feed's icon and the site's language, which a post doesn't have without them.
const POST_META: &str = "site,feed";

/// The stream a Reader menu entry opens: Following, Liked or Discover for the Reader's own, a
/// tag's or a list's posts, and Automattic or P2 for an organization. `None` for an entry that
/// isn't a stream this version knows, or a list whose address doesn't name its owner and slug.
#[uniffi::export]
pub fn reader_stream_for_menu_item(item: &ReaderMenuItem) -> Option<ReaderStream> {
    match &item.kind {
        ReaderMenuItemKind::Stream => match item.key.as_str() {
            "following" => Some(ReaderStream::Following),
            "liked" => Some(ReaderStream::Liked),
            "discover" => Some(ReaderStream::Discover { tags: Vec::new() }),
            _ => None,
        },
        ReaderMenuItemKind::Tag => item.slug.as_ref().map(|slug| ReaderStream::Tag {
            slug: ReaderTagSlug(slug.clone()),
        }),
        ReaderMenuItemKind::List => {
            // A list's owner and slug are only in the address of its posts:
            // `…/read/list/{owner}/{slug}/posts`.
            let url = item.url.as_deref()?;
            let path = url.split(['?', '#']).next().unwrap_or(url);
            let mut segments = path
                .split('/')
                .skip_while(|segment| *segment != "list")
                .skip(1);
            let (owner, slug) = (segments.next()?, segments.next()?);
            (!owner.is_empty() && !slug.is_empty()).then(|| ReaderStream::ReaderList {
                owner: ReaderListOwner(owner.to_string()),
                slug: ReaderListSlug(slug.to_string()),
            })
        }
        ReaderMenuItemKind::Organization => match item.slug.as_deref() {
            Some("a8c") => Some(ReaderStream::Automattic),
            Some("p2") => Some(ReaderStream::P2),
            _ => None,
        },
        ReaderMenuItemKind::Other(_) => None,
    }
}

/// A stream of posts the Reader shows.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderStream {
    /// Posts from the sites and feeds the user follows. Needs an account.
    Following,
    /// The posts the user liked, newest like first. Needs an account.
    ///
    /// The stream is the server's list as last fetched: a post unliked since stays in it, shown
    /// as not liked, until the stream is refreshed.
    Liked,
    /// Posts from Automattic's sites, for Automatticians. Needs an account.
    Automattic,
    /// Posts from the P2s the user follows. Needs an account.
    P2,
    /// Freshly Pressed: the posts WordPress.com's editors picked, newest pick first.
    FreshlyPressed,
    /// A WordPress.com or Jetpack site's posts.
    Site { site_id: WpComSiteId },
    /// An RSS feed's posts.
    Feed { feed_id: ReaderFeedId },
    /// A tag's posts, newest first, in the reader's language.
    Tag { slug: ReaderTagSlug },
    /// The newest posts of several tags, which is Discover's "latest" tab, in the reader's
    /// language.
    Tags { slugs: Vec<ReaderTagSlug> },
    /// A search of posts in the reader's language. `query` is sent whole, `&` and all.
    Search {
        query: String,
        sort: ReaderSearchSort,
    },
    /// A Reader list's posts.
    ReaderList {
        owner: ReaderListOwner,
        slug: ReaderListSlug,
    },
    /// Discover's recommended posts in the reader's language, with cards of sites and tags among
    /// them. `tags` are the reader's interests, for a reader the server doesn't know them for: a
    /// signed-out one.
    Discover { tags: Vec<ReaderTagSlug> },
    /// The first posts of new sites, in the reader's language.
    FirstPosts,
    /// The posts related to a site's post: some from the same site, and some from others. It has
    /// one page.
    Related {
        site_id: WpComSiteId,
        post_id: PostId,
    },
}

/// What orders a stream's posts, which decides how the cache keeps them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderStreamOrdering {
    /// By a date each post has. A page is asked for by date, so a refresh merges into what the
    /// cache has, and a page can be loaded anywhere in the stream: at its end, or into a gap.
    Date,
    /// The order the server sent, paged with a handle the previous page gave. Later pages follow
    /// from the first, so a refresh replaces what the cache has.
    Server,
}

fn tags_key(slugs: &[ReaderTagSlug]) -> String {
    let mut slugs: Vec<&str> = slugs.iter().map(|slug| slug.0.as_str()).collect();
    slugs.sort_unstable();
    slugs.dedup();
    slugs.join(",")
}

impl ReaderStream {
    /// Whether the stream's posts depend on the reader's language. Those streams are sent the
    /// language, and each language is a stream of its own in the cache.
    pub(crate) fn uses_language(&self) -> bool {
        match self {
            Self::Tag { .. }
            | Self::Tags { .. }
            | Self::Search { .. }
            | Self::Discover { .. }
            | Self::FirstPosts => true,
            Self::Following
            | Self::Liked
            | Self::Automattic
            | Self::P2
            | Self::FreshlyPressed
            | Self::Site { .. }
            | Self::Feed { .. }
            | Self::ReaderList { .. }
            | Self::Related { .. } => false,
        }
    }

    /// The language the stream is sent, in the form that picks posts: `None` when the reader has
    /// none, or when the stream's posts don't depend on it.
    fn language(&self, language: Option<ReaderLanguage>) -> Option<&'static str> {
        language
            .filter(|_| self.uses_language())
            .map(|language| language.posts)
    }

    /// The stream's key in the cache, in the reader's language. It never changes for a stream in
    /// a language, since the cache finds the stream's posts by it. The same tags in another order
    /// are the same stream.
    ///
    /// With no language, or for a stream that doesn't depend on it, the key is the stream's own,
    /// as it was before the Reader had a language, so what's cached stays found. In a language, a
    /// stream's key starts with `lang:` and the language, such as `lang:fr:tag:photography`, which
    /// no stream's own key does. A search keeps the language in its own key, where it has always
    /// been: `search:relevance:fr:cats`, or `search:relevance::cats` with none.
    pub(crate) fn key(&self, language: Option<ReaderLanguage>) -> String {
        let language = self.language(language);
        if let Self::Search { query, sort } = self {
            return format!("search:{sort}:{}:{query}", language.unwrap_or_default());
        }
        let key = self.key_in_no_language();
        match language {
            Some(language) => format!("lang:{language}:{key}"),
            None => key,
        }
    }

    fn key_in_no_language(&self) -> String {
        match self {
            Self::Following => "following".to_string(),
            Self::Liked => "liked".to_string(),
            Self::Automattic => "a8c".to_string(),
            Self::P2 => "p2".to_string(),
            Self::FreshlyPressed => "freshly-pressed".to_string(),
            Self::Site { site_id } => format!("site:{}", site_id.0),
            Self::Feed { feed_id } => format!("feed:{}", feed_id.0),
            Self::Tag { slug } => format!("tag:{}", slug.0),
            Self::Tags { slugs } => format!("tags:{}", tags_key(slugs)),
            Self::Search { query, sort } => format!("search:{sort}::{query}"),
            Self::ReaderList { owner, slug } => format!("list:{}/{}", owner.0, slug.0),
            Self::Discover { tags } => format!("discover:{}", tags_key(tags)),
            Self::FirstPosts => "first-posts".to_string(),
            Self::Related { site_id, post_id } => format!("related:{}:{}", site_id.0, post_id.0),
        }
    }

    pub(crate) fn needs_account(&self) -> bool {
        match self {
            Self::Following | Self::Liked | Self::Automattic | Self::P2 => true,
            Self::FreshlyPressed
            | Self::Site { .. }
            | Self::Feed { .. }
            | Self::Tag { .. }
            | Self::Tags { .. }
            | Self::Search { .. }
            | Self::ReaderList { .. }
            | Self::Discover { .. }
            | Self::FirstPosts
            | Self::Related { .. } => false,
        }
    }

    pub(crate) fn ordering(&self) -> ReaderStreamOrdering {
        match self {
            Self::Following
            | Self::Liked
            | Self::Automattic
            | Self::P2
            | Self::FreshlyPressed
            | Self::Site { .. }
            | Self::Feed { .. } => ReaderStreamOrdering::Date,
            Self::Tag { .. }
            | Self::Tags { .. }
            | Self::Search { .. }
            | Self::ReaderList { .. }
            | Self::Discover { .. }
            | Self::FirstPosts
            | Self::Related { .. } => ReaderStreamOrdering::Server,
        }
    }

    /// The date a stream ordered by date orders a post by, and pages by.
    pub(crate) fn sort_date(&self, post: &ReaderPost) -> Option<WpGmtDateTime> {
        match self {
            Self::Liked => post.date_liked,
            Self::FreshlyPressed => post.editorial_displayed_on,
            _ => post.date,
        }
    }

    fn dated(&self, page: ReaderPostsResponse) -> StreamPage {
        StreamPage {
            entries: page
                .posts
                .into_iter()
                .map(|post| {
                    StreamEntry::Post(StreamPost {
                        sort_date: self.sort_date(&post),
                        post,
                    })
                })
                .collect(),
            unreadable: unreadable_posts(page.unreadable_posts),
            next_page_handle: None,
        }
    }

    /// Fetches a page, in `language` for a stream whose posts depend on it: the first page when
    /// `cursor` is `None`.
    pub(crate) async fn fetch(
        &self,
        client: &WpComApiClient,
        cursor: Option<&StreamCursor>,
        language: Option<ReaderLanguage>,
    ) -> Result<StreamPage, WpApiError> {
        let posts = client.reader_posts();
        let locale = self.language(language).map(str::to_string);
        let before = match cursor {
            Some(StreamCursor::Before(date)) => Some(*date),
            _ => None,
        };
        let page_handle = match cursor {
            Some(StreamCursor::Handle(handle)) => Some(handle.clone()),
            _ => None,
        };
        let number = Some(DATED_PAGE_SIZE);
        let meta = Some(POST_META.to_string());
        Ok(match self {
            Self::Following | Self::Site { .. } | Self::Feed { .. } => {
                let params = ReaderStreamParams {
                    number,
                    before,
                    meta,
                    ..Default::default()
                };
                self.dated(match self {
                    Self::Following => posts.following(&params).await?.data,
                    Self::Site { site_id } => posts.site_posts(site_id, &params).await?.data,
                    Self::Feed { feed_id } => posts.feed_posts(feed_id, &params).await?.data,
                    _ => unreachable!("the other streams are fetched below"),
                })
            }
            Self::Liked => {
                let params = ReaderLikedPostsParams {
                    number,
                    before,
                    meta,
                    ..Default::default()
                };
                self.dated(posts.liked(&params).await?.data)
            }
            Self::Automattic | Self::P2 => {
                let params = ReaderOrganizationPostsParams {
                    number,
                    before,
                    meta,
                    ..Default::default()
                };
                self.dated(match self {
                    Self::Automattic => posts.automattic(&params).await?.data,
                    _ => posts.following_p_2(&params).await?.data,
                })
            }
            Self::FreshlyPressed => {
                let params = ReaderFreshlyPressedParams {
                    number,
                    before,
                    meta,
                    ..Default::default()
                };
                self.dated(posts.freshly_pressed(&params).await?.data)
            }
            Self::Tag { slug } => {
                let params = ReaderTagPostsParams {
                    page_handle,
                    sort: Some(ReaderTagPostsSort::Date),
                    locale,
                    ..Default::default()
                };
                tag_page(posts.tag_posts(slug, &params).await?.data)
            }
            Self::Tags { slugs } => {
                let params = ReaderTagsPostsParams {
                    tags: slugs.clone(),
                    number: None,
                    page_handle,
                    page: None,
                    sort: Some(ReaderTagPostsSort::Date),
                    before: None,
                    after: None,
                    locale,
                };
                tag_page(posts.tags_posts(&params).await?.data)
            }
            Self::Search { query, sort } => {
                let params = ReaderSearchPostsParams {
                    q: query.clone(),
                    number: Some(SEARCH_PAGE_SIZE),
                    page_handle,
                    offset: None,
                    sort: Some(*sort),
                    before: None,
                    after: None,
                    lang: locale,
                    algorithm: None,
                    meta,
                    fields: None,
                };
                let page = posts.search(&params).await?.data;
                StreamPage {
                    entries: page
                        .posts
                        .into_iter()
                        .map(|result| undated(result.post))
                        .collect(),
                    unreadable: unreadable_posts(page.unreadable_posts),
                    next_page_handle: page.next_page_handle,
                }
            }
            Self::ReaderList { owner, slug } => {
                let params = ReaderListPostsParams {
                    page_handle,
                    ..Default::default()
                };
                let page = posts.list_posts(owner, slug, &params).await?.data;
                StreamPage {
                    entries: page.posts.into_iter().map(undated).collect(),
                    unreadable: unreadable_posts(page.unreadable_posts),
                    next_page_handle: page.next_page_handle,
                }
            }
            Self::Discover { tags } => {
                let params = ReaderDiscoverParams {
                    count: None,
                    page_handle,
                    tags: tags.clone(),
                    sort: None,
                    age_based_decay: None,
                    locale,
                };
                let page = posts.discover(&params).await?.data;
                card_page(page.cards, page.unreadable_cards, page.next_page_handle)
            }
            Self::FirstPosts => {
                let params = ReaderFirstPostsParams {
                    count: None,
                    page_handle,
                    locale,
                };
                let page = posts.first_posts(&params).await?.data;
                card_page(page.cards, page.unreadable_cards, page.next_page_handle)
            }
            Self::Related { site_id, post_id } => {
                let params = ReaderRelatedPostsParams {
                    size_local: Some(RELATED_POSTS_OF_EACH_KIND),
                    size_global: Some(RELATED_POSTS_OF_EACH_KIND),
                    meta,
                    ..Default::default()
                };
                let page = client
                    .reader_post()
                    .related_posts(site_id, post_id, &params)
                    .await?
                    .data;
                StreamPage {
                    entries: page.posts.into_iter().map(undated).collect(),
                    unreadable: unreadable_posts(page.unreadable_posts),
                    next_page_handle: None,
                }
            }
        })
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
pub fn reader_stream_ordering(stream: ReaderStream) -> ReaderStreamOrdering {
    stream.ordering()
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
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StreamPage {
    /// In the order the server sent.
    pub(crate) entries: Vec<StreamEntry>,
    /// What the page had that couldn't be read: the post's IDs where they could be, and why.
    pub(crate) unreadable: Vec<(Option<ReaderPostSaveIdentity>, String)>,
    /// The handle of the next page of a stream that pages with one, or `None` on its last page.
    pub(crate) next_page_handle: Option<String>,
}

/// Something on a page of a stream: a post, or a card that isn't one.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // A page is nearly all posts.
pub(crate) enum StreamEntry {
    Post(StreamPost),
    Card(ReaderStreamCardKind),
}

/// A post on a page, with the date its stream orders it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamPost {
    pub(crate) post: ReaderPost,
    /// `None` when the post doesn't have the date. A stream ordered by date can't place it then.
    pub(crate) sort_date: Option<WpGmtDateTime>,
}

/// A card in a stream that isn't a post.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReaderStreamCardKind {
    /// Sites the reader may want to follow.
    RecommendedSites { sites: Vec<ReaderStreamCardSite> },
    /// New sites.
    NewSites { sites: Vec<ReaderStreamCardSite> },
    /// Tags the reader may want to follow.
    SuggestedTags { tags: Vec<ReaderStreamCardTag> },
}

/// A site on a card of sites. See [`ReaderCardSite`], which it's stored from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderStreamCardSite {
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
    #[uniffi(default = None)]
    pub icon_url: Option<String>,
    #[uniffi(default = None)]
    pub subscribers_count: Option<u64>,
}

/// A tag on a card of tags. See [`ReaderCardTag`], which it's stored from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct ReaderStreamCardTag {
    pub slug: ReaderTagSlug,
    /// The tag's name, to show.
    #[uniffi(default = None)]
    pub title: Option<String>,
}

fn card_site(site: ReaderCardSite) -> ReaderStreamCardSite {
    ReaderStreamCardSite {
        site_id: site.site_id,
        feed_id: site.feed_id,
        name: site.name,
        description: site.description,
        url: site.url,
        feed_url: site.feed_url,
        icon_url: site.icon_url,
        subscribers_count: site.subscribers_count,
    }
}

fn card_tag(tag: ReaderCardTag) -> ReaderStreamCardTag {
    ReaderStreamCardTag {
        slug: tag.slug,
        title: tag.title,
    }
}

fn undated(post: ReaderPost) -> StreamEntry {
    StreamEntry::Post(StreamPost {
        post,
        sort_date: None,
    })
}

fn unreadable_posts(
    posts: Vec<ReaderUnreadablePost>,
) -> Vec<(Option<ReaderPostSaveIdentity>, String)> {
    posts
        .into_iter()
        .map(|post| (post.identity, post.error))
        .collect()
}

fn tag_page(page: ReaderTagPostsResponse) -> StreamPage {
    StreamPage {
        entries: page.posts.into_iter().map(undated).collect(),
        unreadable: unreadable_posts(page.unreadable_posts),
        next_page_handle: page.next_page_handle,
    }
}

/// A page of a stream of cards. A card of a kind this library doesn't know is left out: nothing
/// could show it.
fn card_page(
    cards: Vec<ReaderCard>,
    unreadable: Vec<ReaderUnreadableCard>,
    next_page_handle: Option<String>,
) -> StreamPage {
    StreamPage {
        entries: cards
            .into_iter()
            .filter_map(|card| match card {
                ReaderCard::Post { post } => Some(undated(post)),
                ReaderCard::RecommendedSites { sites } => {
                    Some(StreamEntry::Card(ReaderStreamCardKind::RecommendedSites {
                        sites: sites.into_iter().map(card_site).collect(),
                    }))
                }
                ReaderCard::NewSites { sites } => {
                    Some(StreamEntry::Card(ReaderStreamCardKind::NewSites {
                        sites: sites.into_iter().map(card_site).collect(),
                    }))
                }
                ReaderCard::SuggestedTags { tags } => {
                    Some(StreamEntry::Card(ReaderStreamCardKind::SuggestedTags {
                        tags: tags.into_iter().map(card_tag).collect(),
                    }))
                }
                ReaderCard::Unknown { .. } => None,
            })
            .collect(),
        unreadable: unreadable
            .into_iter()
            .map(|card| (card.post, card.error))
            .collect(),
        // The API sends an empty handle on some last pages.
        next_page_handle: next_page_handle.filter(|handle| !handle.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use wp_api::wp_com::language::WPComLanguage;

    fn slug(slug: &str) -> ReaderTagSlug {
        ReaderTagSlug(slug.to_string())
    }

    #[rstest]
    #[case(ReaderStream::Following, "following", true, ReaderStreamOrdering::Date)]
    #[case(ReaderStream::Liked, "liked", true, ReaderStreamOrdering::Date)]
    #[case(ReaderStream::Automattic, "a8c", true, ReaderStreamOrdering::Date)]
    #[case(ReaderStream::P2, "p2", true, ReaderStreamOrdering::Date)]
    #[case(
        ReaderStream::FreshlyPressed,
        "freshly-pressed",
        false,
        ReaderStreamOrdering::Date
    )]
    #[case(ReaderStream::Site { site_id: WpComSiteId(3584907) }, "site:3584907", false, ReaderStreamOrdering::Date)]
    #[case(ReaderStream::Feed { feed_id: ReaderFeedId(8857199) }, "feed:8857199", false, ReaderStreamOrdering::Date)]
    #[case(ReaderStream::Tag { slug: slug("photography") }, "tag:photography", false, ReaderStreamOrdering::Server)]
    #[case(ReaderStream::Tags { slugs: vec![slug("travel"), slug("art"), slug("travel")] }, "tags:art,travel", false, ReaderStreamOrdering::Server)]
    #[case(ReaderStream::Search { query: "cats and dogs".to_string(), sort: ReaderSearchSort::Date }, "search:date::cats and dogs", false, ReaderStreamOrdering::Server)]
    #[case(ReaderStream::ReaderList { owner: ReaderListOwner("automattic".to_string()), slug: ReaderListSlug("design".to_string()) }, "list:automattic/design", false, ReaderStreamOrdering::Server)]
    #[case(ReaderStream::Discover { tags: vec![] }, "discover:", false, ReaderStreamOrdering::Server)]
    #[case(ReaderStream::Discover { tags: vec![slug("b"), slug("a")] }, "discover:a,b", false, ReaderStreamOrdering::Server)]
    #[case(
        ReaderStream::FirstPosts,
        "first-posts",
        false,
        ReaderStreamOrdering::Server
    )]
    #[case(ReaderStream::Related { site_id: WpComSiteId(1), post_id: PostId(2) }, "related:1:2", false, ReaderStreamOrdering::Server)]
    fn test_what_each_stream_is(
        #[case] stream: ReaderStream,
        #[case] key: &str,
        #[case] needs_account: bool,
        #[case] ordering: ReaderStreamOrdering,
    ) {
        assert_eq!(stream.key(None), key);
        assert_eq!(stream.needs_account(), needs_account);
        assert_eq!(stream.ordering(), ordering);
    }

    /// Every stream's key in each language. With no language, each key is what it was before the
    /// Reader had a language, so a cache that has them still finds its streams: don't change the
    /// first column.
    #[rstest]
    #[case(ReaderStream::Following, "following", "following")]
    #[case(ReaderStream::Liked, "liked", "liked")]
    #[case(ReaderStream::Automattic, "a8c", "a8c")]
    #[case(ReaderStream::P2, "p2", "p2")]
    #[case(ReaderStream::FreshlyPressed, "freshly-pressed", "freshly-pressed")]
    #[case(ReaderStream::Site { site_id: WpComSiteId(3584907) }, "site:3584907", "site:3584907")]
    #[case(ReaderStream::Feed { feed_id: ReaderFeedId(8857199) }, "feed:8857199", "feed:8857199")]
    #[case(ReaderStream::Tag { slug: slug("photography") }, "tag:photography", "lang:pt:tag:photography")]
    #[case(ReaderStream::Tags { slugs: vec![slug("travel"), slug("art")] }, "tags:art,travel", "lang:pt:tags:art,travel")]
    #[case(ReaderStream::Search { query: "cats".to_string(), sort: ReaderSearchSort::Relevance }, "search:relevance::cats", "search:relevance:pt:cats")]
    #[case(ReaderStream::Search { query: "a:b".to_string(), sort: ReaderSearchSort::Date }, "search:date::a:b", "search:date:pt:a:b")]
    #[case(ReaderStream::ReaderList { owner: ReaderListOwner("automattic".to_string()), slug: ReaderListSlug("design".to_string()) }, "list:automattic/design", "list:automattic/design")]
    #[case(ReaderStream::Discover { tags: vec![] }, "discover:", "lang:pt:discover:")]
    #[case(ReaderStream::Discover { tags: vec![slug("b"), slug("a")] }, "discover:a,b", "lang:pt:discover:a,b")]
    #[case(ReaderStream::FirstPosts, "first-posts", "lang:pt:first-posts")]
    #[case(ReaderStream::Related { site_id: WpComSiteId(1), post_id: PostId(2) }, "related:1:2", "related:1:2")]
    fn test_each_streams_key_with_and_without_a_language(
        #[case] stream: ReaderStream,
        #[case] in_no_language: &str,
        #[case] in_brazilian_portuguese: &str,
    ) {
        assert_eq!(stream.key(None), in_no_language);
        let language = ReaderLanguage::new(WPComLanguage::PortugueseBrazil);
        assert_eq!(stream.key(Some(language)), in_brazilian_portuguese);
        // A stream that depends on the language is another stream in it.
        assert_eq!(
            stream.uses_language(),
            in_no_language != in_brazilian_portuguese
        );
    }

    /// A key in a language can't be any stream's key in no language, whatever its slug or query.
    #[test]
    fn test_a_key_in_a_language_is_no_streams_key_in_none() {
        let french = Some(ReaderLanguage::new(WPComLanguage::French));
        let tag = |slug: &str| ReaderStream::Tag {
            slug: ReaderTagSlug(slug.to_string()),
        };
        assert_ne!(tag("photography").key(french), tag("photography").key(None));
        // No slug makes a key in no language start with `lang:`.
        assert!(
            !tag("lang:fr:tag:photography")
                .key(None)
                .starts_with("lang:")
        );
        let search = |query: &str| ReaderStream::Search {
            query: query.to_string(),
            sort: ReaderSearchSort::Relevance,
        };
        // A query that starts with a colon is still another search than one in French.
        assert_ne!(search("cats").key(french), search(":fr:cats").key(None));
    }

    #[test]
    fn test_a_card_is_stored_and_read_back() {
        let card = ReaderStreamCardKind::RecommendedSites {
            sites: vec![ReaderStreamCardSite {
                site_id: Some(WpComSiteId(1)),
                feed_id: None,
                name: Some("A site".to_string()),
                description: None,
                url: Some("https://example.com".to_string()),
                feed_url: None,
                icon_url: None,
                subscribers_count: Some(12),
            }],
        };
        let stored = serde_json::to_string(&card).unwrap();
        assert_eq!(
            serde_json::from_str::<ReaderStreamCardKind>(&stored).unwrap(),
            card
        );
    }
}
