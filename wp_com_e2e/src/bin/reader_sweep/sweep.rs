//! Which streams a sweep walks, how it pages through one, and when it stops.

use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::str::FromStr;
use wp_api::prelude::WpGmtDateTime;
use wp_api::wp_com::reader_posts::{ReaderPost, ReaderSearchSort};

/// A Reader stream the sweep can walk. More are added as the library gains their endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamKey {
    /// Posts from the sites and feeds the signed-in user follows.
    Following,
    /// The posts the signed-in user liked, newest like first.
    Liked,
    /// Posts from Automattic's sites, for an Automattician.
    Automattic,
    /// Posts from the P2s the signed-in user follows.
    P2,
    /// Freshly Pressed: the posts WordPress.com's editors picked, newest pick first.
    FreshlyPressed,
    /// A WordPress.com site's posts.
    Site(u64),
    /// An RSS feed's posts.
    Feed(u64),
    /// A tag's posts, newest first.
    Tag(String),
    /// The newest posts of several tags, as Discover's "latest" tab shows them.
    Tags(Vec<String>),
    /// A search of posts, in an order.
    Search { sort: SearchSort, query: String },
    /// A Reader list's posts.
    List { owner: String, slug: String },
    /// Discover's cards, leaning toward the given tags when there are any.
    Discover(Vec<String>),
    /// The first posts of new sites, as cards.
    FirstPosts,
}

/// How a search's results are ordered. The library's [`ReaderSearchSort`], in a form a stream's key
/// can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchSort {
    Relevance,
    Date,
}

impl From<SearchSort> for ReaderSearchSort {
    fn from(sort: SearchSort) -> Self {
        match sort {
            SearchSort::Relevance => Self::Relevance,
            SearchSort::Date => Self::Date,
        }
    }
}

/// Where the next page of a stream starts. The streams ordered by a date page by it; the others
/// page by the handle their last page gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    Before(WpGmtDateTime),
    Handle(String),
}

impl StreamKey {
    /// Whether the stream needs a signed-in user.
    pub fn needs_account(&self) -> bool {
        matches!(
            self,
            Self::Following | Self::Liked | Self::Automattic | Self::P2
        )
    }

    /// Whether the stream pages by a date its posts have, rather than by a handle.
    pub fn pages_by_date(&self) -> bool {
        matches!(
            self,
            Self::Following
                | Self::Liked
                | Self::Automattic
                | Self::P2
                | Self::FreshlyPressed
                | Self::Site(_)
                | Self::Feed(_)
        )
    }

    /// The date a stream ordered by date orders a post by, and pages by: when the user liked it on
    /// Liked, when it's shown on Freshly Pressed, and when it was published on the others.
    pub fn sort_date(&self, post: &ReaderPost) -> Option<WpGmtDateTime> {
        match self {
            Self::Liked => post.date_liked,
            Self::FreshlyPressed => post.editorial_displayed_on,
            _ => post.date,
        }
    }
}

impl fmt::Display for StreamKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Following => write!(f, "following"),
            Self::Liked => write!(f, "liked"),
            Self::Automattic => write!(f, "a8c"),
            Self::P2 => write!(f, "p2"),
            Self::FreshlyPressed => write!(f, "freshly-pressed"),
            Self::Site(id) => write!(f, "site:{id}"),
            Self::Feed(id) => write!(f, "feed:{id}"),
            Self::Tag(slug) => write!(f, "tag:{slug}"),
            Self::Tags(slugs) => write!(f, "tags:{}", slugs.join(",")),
            Self::Search { sort, query } => {
                write!(f, "search:{}:{query}", ReaderSearchSort::from(*sort))
            }
            Self::List { owner, slug } => write!(f, "list:{owner}/{slug}"),
            Self::Discover(tags) if tags.is_empty() => write!(f, "discover"),
            Self::Discover(tags) => write!(f, "discover:{}", tags.join(",")),
            Self::FirstPosts => write!(f, "first-posts"),
        }
    }
}

const STREAM_FORMS: &str = "`following`, `liked`, `a8c`, `p2`, `freshly-pressed`, `site:<id>`, \
     `feed:<id>`, `tag:<slug>`, `tags:<slug>,<slug>`, `search:<relevance|date>:<query>`, \
     `list:<owner>/<slug>`, `discover`, `discover:<slug>,<slug>` or `first-posts`";

impl FromStr for StreamKey {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        match text {
            "following" => return Ok(Self::Following),
            "liked" => return Ok(Self::Liked),
            "a8c" => return Ok(Self::Automattic),
            "p2" => return Ok(Self::P2),
            "freshly-pressed" => return Ok(Self::FreshlyPressed),
            "discover" => return Ok(Self::Discover(Vec::new())),
            "first-posts" => return Ok(Self::FirstPosts),
            _ => {}
        }
        let id = |value: &str| {
            value
                .parse::<u64>()
                .ok()
                .filter(|id| *id > 0)
                .ok_or_else(|| format!("`{text}` needs a positive ID"))
        };
        let slugs = |value: &str| {
            let slugs: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|slug| !slug.is_empty())
                .map(str::to_string)
                .collect();
            match slugs.is_empty() {
                true => Err(format!("`{text}` needs at least one tag")),
                false => Ok(slugs),
            }
        };
        let not_a_stream = || format!("`{text}` isn't a stream: use {STREAM_FORMS}");
        match text.split_once(':') {
            Some(("site", value)) => Ok(Self::Site(id(value)?)),
            Some(("feed", value)) => Ok(Self::Feed(id(value)?)),
            Some(("tag", slug)) if !slug.trim().is_empty() => {
                Ok(Self::Tag(slug.trim().to_string()))
            }
            Some(("tags", value)) => Ok(Self::Tags(slugs(value)?)),
            Some(("discover", value)) => Ok(Self::Discover(slugs(value)?)),
            Some(("search", rest)) => {
                let (sort, query) = rest.split_once(':').ok_or_else(not_a_stream)?;
                let sort = match sort.parse::<ReaderSearchSort>() {
                    Ok(ReaderSearchSort::Relevance) => SearchSort::Relevance,
                    Ok(ReaderSearchSort::Date) => SearchSort::Date,
                    Err(_) => return Err(not_a_stream()),
                };
                match query.trim().is_empty() {
                    true => Err(format!("`{text}` needs something to search for")),
                    false => Ok(Self::Search {
                        sort,
                        query: query.trim().to_string(),
                    }),
                }
            }
            Some(("list", rest)) => match rest.split_once('/') {
                Some((owner, slug)) if !owner.trim().is_empty() && !slug.trim().is_empty() => {
                    Ok(Self::List {
                        owner: owner.trim().to_string(),
                        slug: slug.trim().to_string(),
                    })
                }
                _ => Err(not_a_stream()),
            },
            _ => Err(not_a_stream()),
        }
    }
}

/// Streams from a seeds file: one per line, with blank lines and `#` comments skipped.
pub fn parse_seeds(text: &str) -> Result<Vec<StreamKey>, String> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter(|line| !line.is_empty())
        .map(StreamKey::from_str)
        .collect()
}

/// The streams a post points to: its site's and its feed's.
///
/// For an RSS post the API puts the feed's ID in `site_ID`, so a site is only taken from a
/// WordPress.com or Jetpack post.
pub fn streams_of(post: &ReaderPost) -> Vec<StreamKey> {
    let mut streams = Vec::new();
    let has_site = !post.identity.is_external || post.is_jetpack == Some(true);
    if has_site && let Some(site) = post.identity.site_id.filter(|site| site.0 > 0) {
        streams.push(StreamKey::Site(site.0));
    }
    if let Some(feed) = post.identity.feed_id.filter(|feed| feed.0 > 0) {
        streams.push(StreamKey::Feed(feed.0));
    }
    streams
}

/// Where the next page of a date-paged stream starts, or `None` when the stream is done.
///
/// The next page is the posts before the oldest date on this page: `dates` are the date the stream
/// orders each post by. A page without a date, or whose oldest date isn't older than where this
/// page started, wouldn't move on, so it ends the stream.
pub fn next_before(
    dates: impl IntoIterator<Item = Option<WpGmtDateTime>>,
    current_before: Option<WpGmtDateTime>,
) -> Option<WpGmtDateTime> {
    let oldest = dates.into_iter().flatten().min_by_key(|date| date.0)?;
    match current_before {
        Some(current) if oldest.0 >= current.0 => None,
        _ => Some(oldest),
    }
}

/// The streams waiting to be walked, and every stream the sweep has seen, so none is walked twice.
pub struct Queue {
    waiting: VecDeque<StreamKey>,
    seen: HashSet<StreamKey>,
    max_streams: usize,
}

impl Queue {
    pub fn new(seeds: Vec<StreamKey>, max_streams: usize) -> Self {
        let mut queue = Self {
            waiting: VecDeque::new(),
            seen: HashSet::new(),
            max_streams,
        };
        for seed in seeds {
            queue.add(seed);
        }
        queue
    }

    /// Adds a stream unless it's been seen, or the sweep already has as many as it may walk.
    pub fn add(&mut self, stream: StreamKey) -> bool {
        if self.seen.len() >= self.max_streams || self.seen.contains(&stream) {
            return false;
        }
        self.seen.insert(stream.clone());
        self.waiting.push_back(stream);
        true
    }

    pub fn next(&mut self) -> Option<StreamKey> {
        self.waiting.pop_front()
    }

    pub fn seen(&self) -> usize {
        self.seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_api::wp_com::WpComSiteId;
    use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderPostSaveIdentity};

    fn post(json: &str) -> ReaderPost {
        serde_json::from_str(json).expect("a Reader post")
    }

    fn dated(date: &str) -> ReaderPost {
        post(&format!(r#"{{"ID": 1, "site_ID": 2, "date": "{date}"}}"#))
    }

    fn at(date: &str) -> WpGmtDateTime {
        dated(date).date.expect("a date")
    }

    #[test]
    fn stream_keys_round_trip() {
        for text in [
            "following",
            "site:3584907",
            "feed:8857199",
            "tag:photography",
            "tag:写真",
            "liked",
            "a8c",
            "p2",
            "freshly-pressed",
            "tags:photography,travel",
            "search:relevance:cats and dogs",
            "search:date:a: b",
            "list:automattic/a12s",
            "discover",
            "discover:art,travel",
            "first-posts",
        ] {
            assert_eq!(text.parse::<StreamKey>().unwrap().to_string(), text);
        }
    }

    #[test]
    fn a_stream_key_needs_a_kind_and_a_positive_id() {
        for text in [
            "",
            "site",
            "site:",
            "site:0",
            "site:-1",
            "site:abc",
            "tag:",
            "tag: ",
            "list:wordpress",
            "list:/a12s",
            "list:automattic/",
            "tags:",
            "tags:,",
            "discover:",
            "search:cats",
            "search:popular:cats",
            "search:date:",
            "42",
        ] {
            assert!(text.parse::<StreamKey>().is_err(), "{text}");
        }
    }

    #[test]
    fn the_streams_that_need_an_account_and_how_each_pages() {
        let key = |text: &str| text.parse::<StreamKey>().unwrap();
        for text in ["following", "liked", "a8c", "p2"] {
            assert!(key(text).needs_account(), "{text}");
            assert!(key(text).pages_by_date(), "{text}");
        }
        for text in ["freshly-pressed", "site:1", "feed:1"] {
            assert!(!key(text).needs_account(), "{text}");
            assert!(key(text).pages_by_date(), "{text}");
        }
        for text in [
            "tag:a",
            "tags:a,b",
            "search:date:a",
            "list:a/b",
            "discover",
            "first-posts",
        ] {
            assert!(!key(text).needs_account(), "{text}");
            assert!(!key(text).pages_by_date(), "{text}");
        }
    }

    #[test]
    fn liked_and_freshly_pressed_page_by_their_own_dates() {
        let post = post(
            r#"{"ID": 1, "site_ID": 2, "date": "2026-01-01T00:00:00+00:00", "date_liked": "2026-02-01T00:00:00+00:00", "editorial": {"displayed_on": "2026-03-01T00:00:00+00:00"}}"#,
        );
        assert_eq!(
            StreamKey::Liked.sort_date(&post),
            Some(at("2026-02-01T00:00:00+00:00"))
        );
        assert_eq!(
            StreamKey::FreshlyPressed.sort_date(&post),
            Some(at("2026-03-01T00:00:00+00:00"))
        );
        assert_eq!(
            StreamKey::Following.sort_date(&post),
            Some(at("2026-01-01T00:00:00+00:00"))
        );
    }

    #[test]
    fn a_seeds_file_skips_blank_lines_and_comments() {
        let seeds =
            parse_seeds("# public sites\nsite:3584907\n\n  feed:8857199  # xkcd\nfollowing\n");
        assert_eq!(
            seeds,
            Ok(vec![
                StreamKey::Site(3584907),
                StreamKey::Feed(8857199),
                StreamKey::Following
            ])
        );
        assert!(parse_seeds("site:1\nnonsense\n").is_err());
    }

    #[test]
    fn a_wordpress_com_post_points_to_its_site_and_feed() {
        let post =
            post(r#"{"ID": 10, "site_ID": 3584907, "feed_ID": 25823, "is_external": false}"#);
        assert_eq!(
            streams_of(&post),
            vec![StreamKey::Site(3584907), StreamKey::Feed(25823)]
        );
    }

    #[test]
    fn an_rss_post_points_only_to_its_feed() {
        // The API sends the feed's ID as `site_ID` for an RSS post.
        let post = post(
            r#"{"ID": 10, "site_ID": 8857199, "feed_ID": 8857199, "feed_item_ID": 10, "is_external": true, "is_jetpack": false}"#,
        );
        assert_eq!(streams_of(&post), vec![StreamKey::Feed(8857199)]);
    }

    #[test]
    fn a_jetpack_post_points_to_its_site_and_feed() {
        let post = post(
            r#"{"ID": 10, "site_ID": 59473028, "feed_ID": 77, "feed_item_ID": 10, "is_external": true, "is_jetpack": true}"#,
        );
        assert_eq!(
            streams_of(&post),
            vec![StreamKey::Site(59473028), StreamKey::Feed(77)]
        );
    }

    #[test]
    fn a_post_without_ids_points_nowhere() {
        let identity = ReaderPostSaveIdentity {
            id: 10,
            site_id: Some(WpComSiteId(0)),
            feed_id: Some(ReaderFeedId(0)),
            feed_item_id: None,
            is_external: false,
            url: None,
        };
        let mut post = post(r#"{"ID": 10}"#);
        post.identity = identity;
        assert_eq!(streams_of(&post), vec![]);
    }

    #[test]
    fn the_next_page_starts_before_the_oldest_post() {
        let posts = [
            dated("2026-11-03T10:00:00+00:00"),
            dated("2026-11-01T10:00:00+00:00"),
            dated("2026-11-02T10:00:00+00:00"),
        ];
        let dates = || posts.iter().map(|post| post.date);
        assert_eq!(
            next_before(dates(), None),
            Some(at("2026-11-01T10:00:00+00:00"))
        );
        assert_eq!(
            next_before(dates(), Some(at("2026-11-04T00:00:00+00:00"))),
            Some(at("2026-11-01T10:00:00+00:00"))
        );
    }

    #[test]
    fn a_page_that_would_not_move_on_ends_the_stream() {
        let date = || Some(at("2026-11-01T10:00:00+00:00"));
        // The same date the page started from, and a newer one.
        assert_eq!(
            next_before([date()], Some(at("2026-11-01T10:00:00+00:00"))),
            None
        );
        assert_eq!(
            next_before([date()], Some(at("2026-10-31T10:00:00+00:00"))),
            None
        );
        // No posts, and posts without a date.
        assert_eq!(next_before([], None), None);
        assert_eq!(next_before([None], None), None);
    }

    #[test]
    fn the_queue_walks_each_stream_once_and_stops_at_its_limit() {
        let mut queue = Queue::new(vec![StreamKey::Following, StreamKey::Following], 3);
        assert_eq!(queue.seen(), 1);
        assert!(queue.add(StreamKey::Site(1)));
        assert!(!queue.add(StreamKey::Site(1)));
        assert!(queue.add(StreamKey::Feed(1)));
        // The limit counts streams seen, walked or not.
        assert!(!queue.add(StreamKey::Site(2)));
        assert_eq!(queue.next(), Some(StreamKey::Following));
        assert_eq!(queue.next(), Some(StreamKey::Site(1)));
        assert_eq!(queue.next(), Some(StreamKey::Feed(1)));
        assert_eq!(queue.next(), None);
        assert!(!queue.add(StreamKey::Site(3)));
    }
}
