//! Which streams a sweep walks, how it pages through one, and when it stops.

use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use wp_api::prelude::WpGmtDateTime;
use wp_api::wp_com::reader_posts::ReaderPost;

/// A Reader stream the sweep can walk. More are added as the library gains their endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamKey {
    /// Posts from the sites and feeds the signed-in user follows.
    Following,
    /// A WordPress.com site's posts.
    Site(u64),
    /// An RSS feed's posts.
    Feed(u64),
    /// A tag's posts, newest first.
    Tag(String),
}

/// Where the next page of a stream starts. Following, a site and a feed page by date; a tag pages by
/// the handle its last page gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    Before(WpGmtDateTime),
    Handle(String),
}

impl fmt::Display for StreamKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Following => write!(f, "following"),
            Self::Site(id) => write!(f, "site:{id}"),
            Self::Feed(id) => write!(f, "feed:{id}"),
            Self::Tag(slug) => write!(f, "tag:{slug}"),
        }
    }
}

impl FromStr for StreamKey {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text == "following" {
            return Ok(Self::Following);
        }
        let id = |value: &str| {
            value
                .parse::<u64>()
                .ok()
                .filter(|id| *id > 0)
                .ok_or_else(|| format!("`{text}` needs a positive ID"))
        };
        match text.split_once(':') {
            Some(("site", value)) => Ok(Self::Site(id(value)?)),
            Some(("feed", value)) => Ok(Self::Feed(id(value)?)),
            Some(("tag", slug)) if !slug.trim().is_empty() => {
                Ok(Self::Tag(slug.trim().to_string()))
            }
            _ => Err(format!(
                "`{text}` isn't a stream: use `following`, `site:<id>`, `feed:<id>` or `tag:<slug>`"
            )),
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
/// The next page is the posts before the oldest one on this page. A page without a date, or whose
/// oldest post isn't older than where this page started, wouldn't move on, so it ends the stream.
pub fn next_before(
    posts: &[ReaderPost],
    current_before: Option<WpGmtDateTime>,
) -> Option<WpGmtDateTime> {
    let oldest = posts
        .iter()
        .filter_map(|post| post.date)
        .min_by_key(|date| date.0)?;
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

/// Refuses an output directory inside a Git checkout.
///
/// A run holds real posts, some of them private, and they mustn't end up committed.
pub fn check_output_dir(dir: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let mut ancestor = Some(absolute.as_path());
    while let Some(path) = ancestor {
        // A worktree has a `.git` file rather than a directory.
        if path.join(".git").exists() {
            return Err(format!(
                "{} is inside the Git checkout at {}. A run holds real posts, some of them \
                 private, so choose a directory outside any checkout.",
                absolute.display(),
                path.display()
            ));
        }
        ancestor = path.parent();
    }
    Ok(absolute)
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
            "42",
        ] {
            assert!(text.parse::<StreamKey>().is_err(), "{text}");
        }
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
        assert_eq!(
            next_before(&posts, None),
            Some(at("2026-11-01T10:00:00+00:00"))
        );
        assert_eq!(
            next_before(&posts, Some(at("2026-11-04T00:00:00+00:00"))),
            Some(at("2026-11-01T10:00:00+00:00"))
        );
    }

    #[test]
    fn a_page_that_would_not_move_on_ends_the_stream() {
        let posts = [dated("2026-11-01T10:00:00+00:00")];
        // The same date the page started from, and a newer one.
        assert_eq!(
            next_before(&posts, Some(at("2026-11-01T10:00:00+00:00"))),
            None
        );
        assert_eq!(
            next_before(&posts, Some(at("2026-10-31T10:00:00+00:00"))),
            None
        );
        // No posts, and posts without a date.
        assert_eq!(next_before(&[], None), None);
        assert_eq!(next_before(&[post(r#"{"ID": 1}"#)], None), None);
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

    #[test]
    fn an_output_directory_inside_a_checkout_is_refused() {
        let inside = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/reader-sweep");
        assert!(
            check_output_dir(&inside)
                .unwrap_err()
                .contains("Git checkout")
        );
        let outside = std::env::temp_dir().join("reader-sweep-outside");
        assert_eq!(
            check_output_dir(&outside),
            Ok(std::path::absolute(&outside).unwrap())
        );
    }
}
