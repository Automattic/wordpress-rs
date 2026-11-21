//! Links: what a post's, a site's or a feed's address opens in the Reader.
//!
//! The server has no one request for this. What each one answers, as found live:
//!
//! - `GET /wpcom/v2/mobile/resolve-reader-url` resolves a post's address on a WordPress.com or
//!   Jetpack site to its IDs, on the site's own domain too, with a query or fragment or without.
//!   It needs a token: signed out, every address is a 401 `rest_forbidden`. A site's home address
//!   is a 400, whether signed in or not: the API wants a path. Any other address on a WordPress.com
//!   or Jetpack site, such as a page that isn't there, a category, a feed or a `wp.me` short link,
//!   is a 404 `post_not_found`. An address on any other site is a 404 `blog_not_found` (seen
//!   from 2026-11-20 20:17 UTC; until 19:46 UTC that day it was a 500 with no body).
//! - `GET /rest/v1.2/read/sites/{domain}` answers for a WordPress.com or Jetpack site's domain,
//!   signed out too, and is a 404 `unknown_blog` for any other domain.
//! - `GET /rest/v1.1/read/feed?url=` finds the feeds at an address, on any site, signed out too,
//!   and is a 404 `no_such_feed` where it finds none, as at a page that isn't there.
//!
//! The Reader's own pages on `wordpress.com` name a site, a feed or a post by its IDs, so they're
//! read here rather than sent to the resolver. The shapes, from the web Reader's routes and its
//! redirects of older addresses (Calypso's `client/reader/index.ts`, `full-post/index.js` and
//! `setupReadRoutes` in `controller.jsx`, 2026-11-18) and the Android app's
//! `ReaderPostPagerActivity` and `ReaderLinkHandler` (2026-11-17), each under `/read/` or
//! `/reader/`:
//!
//! - `blogs/{site}` and `blogs/{site}/posts`: a site.
//! - `feeds/{feed}` and `feeds/{feed}/posts`: a feed. The web Reader opens a feed of a
//!   WordPress.com or Jetpack site as the site.
//! - `blogs/{site}/posts/{post}`: a site's post.
//! - `feeds/{feed}/posts/{item}`: a feed's post.
//! - Under `/read/` only, older addresses the web redirects: `blog/id/{site}`, `blog/feed/{feed}`,
//!   `post/id/{site}/{post}` and `post/feed/{feed}/{item}`.
//!
//! The IDs are numbers, as the web's routes require. Asked about with IDs that name nothing,
//! `read/sites/{id}` is a 404 `unknown_blog` (410 `site_gone` for a deleted site),
//! `read/feed/{id}` a 404 `no_such_feed`, a site's post a 404 `unknown_post` and a feed's item a
//! 404 `no_such_item`.
//!
//! The Reader's pages that open a stream that isn't a site's or a feed's are read here too, and
//! need no request. Their shapes, from the web's routes and redirects (Calypso's
//! `client/reader/index.ts`, `controller.jsx`'s `setupReadRoutes`, `tag-stream/`, `discover/` and
//! `liked-stream/`, 2026-11-18), Android's `ReaderLinkHandler` and manifest, and iOS's
//! `ReaderRoute` (both 2026-11-17):
//!
//! - Following: `read`, `reader`, `read/following`, `reader/following` and `following`.
//! - Liked: `activities/likes`. Automattic: `read/a8c`, `reader/a8c`. P2: `read/p2`, `reader/p2`.
//!   These need an account, as their streams do.
//! - A tag: `tag/{slug}`, `tag/#{slug}` (the web moves the fragment into the path), `read/tag/{slug}`
//!   (the web redirects it), and `read/tags/{slug}` and `reader/tags/{slug}`, the API's own form,
//!   which nothing routes but a reader may paste. The slug is read as the web reads it: decoded,
//!   trimmed, in lower case, with spaces as dashes. It must then be letters, digits, `_` and `-`,
//!   or characters outside ASCII, as Android's `ReaderUtils.isValidTagSlug` requires. The web's
//!   `?sort=relevance` (its popular posts of a tag) isn't a stream the library has, so it's not read.
//! - A list: `read/list/{owner}/{slug}`, `reader/list/{owner}/{slug}`, and
//!   `reader/list/{owner}/{slug}/{view}` but for the `edit`, `export` and `delete` pages.
//! - A search: `read/search`, `reader/search`, `discover/search` and `recommendations`, with the
//!   query in `q` and `sort=date` for the newest first; any other sort is the web's default, best
//!   match. With no query, it's the search page: [`ReaderLink::Search`].
//! - Discover: `discover`, `discover/recommended`, `read/discover` and `reader/discover`.
//!   `discover/tags?selectedTag={slug}` is Discover of that one tag, as the web asks for it
//!   (`tags={slug}`), and of `dailyprompt` with none, the web's default.
//!   `discover/freshly-pressed` is Freshly Pressed. `discover/latest` is the latest posts of the
//!   reader's tags, which the address doesn't name, so it opens Discover.
//! - `tag`, `discover`, `read` and `reader` can follow a language, such as `fr/tag/{slug}`. The
//!   stream is in the reader's language, as [`ReaderService::set_language`] sets it, not the
//!   address's.
//!
//! Neither app reads a search's query or Discover's tag from the address: Android opens its search
//! and Discover screens, iOS too, and both open a tag by its slug. The library follows the web,
//! which does read them.

use super::actions::is_refused;
use super::{ReaderError, ReaderService, ReaderStream};
use wp_api::prelude::{WpApiError, WpErrorCode};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_post::{ReaderPostBySlugParams, ReaderPostSlug, ReaderResolveUrlParams};
use wp_api::wp_com::reader_posts::{
    ReaderListOwner, ReaderListSlug, ReaderSearchSort, ReaderTagSlug,
};
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderFeedItemId, ReaderPostSaveIdentity};
use wp_api::wp_com::reader_sites::{ReaderFeedSearchParams, ReaderSiteParams};
use wp_api::wp_com::sites::WpComSiteIdentifier;
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
use wp_mobile_cache::repository::reader_sites::ReaderSiteRecord;
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

/// The host of WordPress.com's short links.
const SHORT_LINK_HOST: &str = "wp.me";

/// What a link opens in the Reader.
#[allow(clippy::large_enum_variant)] // Boxing doesn't cross the FFI boundary.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderLink {
    /// A post, now in the cache with its content.
    Post { post: ReaderPostRecord },
    /// A WordPress.com or Jetpack site: open its [`super::ReaderStream::Site`]. `site` is what the
    /// cache now has of it.
    Site {
        site_id: WpComSiteId,
        site: ReaderSiteRecord,
    },
    /// A feed of a site that isn't on WordPress.com or Jetpack: open its
    /// [`super::ReaderStream::Feed`]. `site` is what the cache now has of it.
    Feed {
        feed_id: ReaderFeedId,
        site: ReaderSiteRecord,
    },
    /// One of the Reader's own pages of a stream that isn't a site's or a feed's: a tag's, a
    /// list's, a search, Discover, Freshly Pressed, or one of the account's streams. Open
    /// `stream`. Nothing was sent.
    ///
    /// A [`ReaderStream::Discover`] from an address has no tags: open it as Discover is opened
    /// from the menu, with the reader's interests when signed out.
    Stream { stream: ReaderStream },
    /// The Reader's search page, with no query in the address: open the search screen. Nothing was
    /// sent.
    Search,
    /// Nothing the Reader can show.
    NotInReader { reason: ReaderLinkNotInReader },
}

/// Why a link opens nothing in the Reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ReaderLinkNotInReader {
    /// It isn't a web address: it needs `http` or `https`, and a host. Nothing was sent.
    NotAnAddress,
    /// A `wp.me` short link, which the server doesn't resolve. Nothing was sent. A browser can
    /// open it.
    ShortLink,
    /// The address is on a WordPress.com or Jetpack site, and the server has no post there and no
    /// feed, such as a page that isn't there. Or it's one of the Reader's own addresses, and it
    /// names nothing the Reader has: a site, feed or post the server doesn't have, or a page that
    /// isn't a stream, such as the subscriptions manager.
    NothingThere,
    /// There's no WordPress.com or Jetpack site at the address, and no feed.
    NotWordPressOrFeed,
}

/// What a link that opens nothing in the Reader says, in words, for an app to show.
#[uniffi::export]
pub fn reader_link_not_in_reader_message(reason: ReaderLinkNotInReader) -> String {
    match reason {
        ReaderLinkNotInReader::NotAnAddress => "That isn't a web address.",
        ReaderLinkNotInReader::ShortLink => {
            "The Reader can't open a wp.me short link. Open it in a browser."
        }
        ReaderLinkNotInReader::NothingThere => {
            "There's nothing at that address for the Reader to show."
        }
        ReaderLinkNotInReader::NotWordPressOrFeed => {
            "That address isn't a WordPress.com or Jetpack site, and has no feed the Reader can show."
        }
    }
    .to_string()
}

/// Whether an error is the server's answer `code`.
fn is_code(error: &WpApiError, code: &str) -> bool {
    matches!(
        error,
        WpApiError::WpError { error_code: WpErrorCode::CustomError(found), .. } if found == code
    )
}

/// Reads a link the way someone would type or paste it. One without a scheme is taken to be
/// `https`. `None` for anything that isn't an `http` or `https` address with a host.
fn parse_link(link: &str) -> Option<url::Url> {
    let link = link.trim();
    let parsed = match url::Url::parse(link) {
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            url::Url::parse(&format!("https://{link}")).ok()?
        }
        parsed => parsed.ok()?,
    };
    let is_web = matches!(parsed.scheme(), "http" | "https");
    (is_web && parsed.host_str().is_some_and(|host| !host.is_empty())).then_some(parsed)
}

/// Decodes a path segment's `%XX` escapes. `None` when what they spell isn't UTF-8.
fn percent_decoded(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = (bytes[at] == b'%')
            .then(|| bytes.get(at + 1..at + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                at += 3;
            }
            None => {
                decoded.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

/// The slug of the post at an address: the last part of its path, decoded. `None` when that part
/// can't be a post's slug, so there's nothing to look up. WordPress never gives a post a slug of
/// only digits, `page` and digits or a feed's name (`wp_unique_post_slug`), nor one with a `.`
/// (`sanitize_title_with_dashes`), so such a part is an archive page, a feed or a file.
///
/// It's how the Android app opens a post's address without an account: the host is the site, and
/// the slug what follows the date in `/{year}/{month}/{day}/{slug}/`. Taking the last part of the
/// path covers that and other permalinks. Measured signed out on 2026-11-20, looking up
/// `sites/{host}/posts/slug:{slug}` for real posts' addresses:
///
/// - WordPress.com sites: 45 of 45 Discover posts (`/{year}/{month}/{day}/{slug}/`, on
///   `*.wordpress.com` and mapped domains) and 16 of 16 posts of eight well-known sites
///   (`/{year}/{month}/{slug}/`, `/{slug}/` under a path such as `/blog/` or `/posts/`) were
///   found, with the post's own IDs.
/// - Jetpack sites: 12 of 14 posts of seven sites (`/{slug}/` and dated). The two not found
///   (404 `unknown_post`) were under `/recipes/`, likely another post type.
/// - A site in a folder of its domain, such as `wordpress.org/news/`: 0 of 2. The host is another
///   site, which answers `unknown_post`.
/// - A slug the site doesn't have is a 404 `unknown_post`, a host that's no WordPress.com or
///   Jetpack site's a 404 `unknown_blog`, and a private site a 403 `unauthorized`.
fn post_slug(url: &url::Url) -> Option<String> {
    let last = url.path_segments()?.rev().find(|part| !part.is_empty())?;
    let slug = percent_decoded(last)?;
    let lower = slug.to_lowercase();
    let digits = lower.strip_prefix("page").unwrap_or(&lower);
    let is_numeric = !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
    let is_feed = matches!(lower.as_str(), "feed" | "rdf" | "rss" | "rss2" | "atom");
    (!is_numeric && !is_feed && !slug.contains('.') && !slug.trim().is_empty()).then_some(slug)
}

/// The hosts the Reader's own pages are on.
const READER_HOSTS: [&str; 2] = ["wordpress.com", "www.wordpress.com"];

/// What one of the Reader's own addresses names. See the module's documentation for the shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReaderAddress {
    Site(WpComSiteId),
    Feed(ReaderFeedId),
    SitePost(WpComSiteId, u64),
    FeedPost(ReaderFeedId, ReaderFeedItemId),
    /// A stream that isn't a site's or a feed's.
    Stream(ReaderStream),
    /// The search page, with no query.
    SearchPage,
    /// A Reader address that names no site, feed or post.
    Nothing,
}

/// An ID in a Reader address: a number above zero, in digits only.
fn reader_id(segment: &str) -> Option<u64> {
    if segment.is_empty() || !segment.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    segment.parse().ok().filter(|id| *id != 0)
}

/// The words a Reader address starts with, after any language.
const READER_WORDS: [&str; 7] = [
    "read",
    "reader",
    "tag",
    "discover",
    "following",
    "activities",
    "recommendations",
];

/// Whether a path's first part is a language, as the web puts one before some Reader pages:
/// `fr`, `pt-br`, `zh-tw`.
fn is_language(word: &str) -> bool {
    let (language, region) = word.split_once('-').unwrap_or((word, ""));
    let letters = |part: &str, sizes: std::ops::RangeInclusive<usize>| {
        sizes.contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_lowercase())
    };
    letters(language, 2..=3) && (region.is_empty() || letters(region, 2..=4))
}

/// A tag's slug as it's written in an address, read as the web reads it (Calypso's
/// `tag-stream/controller.jsx`): decoded, trimmed, in lower case, with spaces as dashes and no
/// two dashes together. `None` unless what's left is letters, digits, `_` and `-`, or characters
/// outside ASCII that aren't spaces or controls, as Android's `ReaderUtils.isValidTagSlug` requires.
fn tag_slug(written: &str) -> Option<ReaderTagSlug> {
    let decoded = percent_decoded(written)?.trim().to_lowercase();
    let mut slug = String::with_capacity(decoded.len());
    for character in decoded.chars() {
        let character = if character.is_whitespace() {
            '-'
        } else {
            character
        };
        if !(character == '-' && slug.ends_with('-')) {
            slug.push(character);
        }
    }
    let allowed = |character: char| {
        character.is_ascii_alphanumeric()
            || matches!(character, '_' | '-')
            || (!character.is_ascii() && !character.is_whitespace() && !character.is_control())
    };
    (!slug.is_empty() && slug.chars().all(allowed)).then_some(ReaderTagSlug(slug))
}

/// The value of a query parameter, decoded as a browser decodes it, when it isn't blank.
fn query_value(url: &url::Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// A search page: the search of its `q` and `sort`, or the search page when it has no query.
fn search_address(url: &url::Url) -> ReaderAddress {
    match query_value(url, "q") {
        Some(query) => ReaderAddress::Stream(ReaderStream::Search {
            query,
            sort: match query_value(url, "sort").as_deref() {
                Some("date") => ReaderSearchSort::Date,
                _ => ReaderSearchSort::Relevance,
            },
        }),
        None => ReaderAddress::SearchPage,
    }
}

/// What a Reader address names, or `None` when the address isn't one of the Reader's: on
/// another host, or with a path that doesn't start with one of [`READER_WORDS`], after any
/// language. `host` is in lower case.
fn reader_address(host: &str, url: &url::Url) -> Option<ReaderAddress> {
    if !READER_HOSTS.contains(&host) {
        return None;
    }
    // The web matches the words of a Reader address in any case, and the IDs as digits.
    let segments: Vec<&str> = url
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect();
    let lower: Vec<String> = segments
        .iter()
        .map(|word| word.to_ascii_lowercase())
        .collect();
    let mut words: Vec<&str> = lower.iter().map(String::as_str).collect();
    let mut written = segments;
    if words.len() > 1 && is_language(words[0]) && READER_WORDS.contains(&words[1]) {
        words.remove(0);
        written.remove(0);
    }
    let first = *words.first()?;
    if !READER_WORDS.contains(&first) {
        return None;
    }
    let stream = |stream: ReaderStream| Some(ReaderAddress::Stream(stream));
    let discover = || stream(ReaderStream::Discover { tags: Vec::new() });
    let tag = |slug: &str| tag_slug(slug).and_then(|slug| stream(ReaderStream::Tag { slug }));
    let list = |owner: &str, slug: &str| {
        let owner = percent_decoded(owner)?;
        let slug = percent_decoded(slug)?;
        stream(ReaderStream::ReaderList {
            owner: ReaderListOwner(owner),
            slug: ReaderListSlug(slug),
        })
    };
    let site = |id: &str| reader_id(id).map(WpComSiteId);
    let feed = |id: &str| reader_id(id).map(ReaderFeedId);
    let named = match words.as_slice() {
        ["read" | "reader"] | ["read" | "reader" | "following", "following"] | ["following"] => {
            stream(ReaderStream::Following)
        }
        ["activities", "likes"] => stream(ReaderStream::Liked),
        ["read" | "reader", "a8c"] => stream(ReaderStream::Automattic),
        ["read" | "reader", "p2"] => stream(ReaderStream::P2),
        // `/tag/#cats`: the slug is the fragment.
        ["tag"] => url.fragment().and_then(tag),
        ["tag", _] => tag(written[1]),
        ["read", "tag", _] | ["read" | "reader", "tags", _] => tag(written[2]),
        ["read" | "reader", "list", _, _] => list(written[2], written[3]),
        ["reader", "list", _, _, view] if !matches!(*view, "edit" | "export" | "delete") => {
            list(written[2], written[3])
        }
        ["read" | "reader" | "discover", "search"] | ["recommendations"] => {
            Some(search_address(url))
        }
        ["discover"] | ["discover", "recommended" | "latest"] | ["read" | "reader", "discover"] => {
            discover()
        }
        ["discover", "freshly-pressed"] => stream(ReaderStream::FreshlyPressed),
        ["discover", "tags"] => match url.query_pairs().find(|(key, _)| key == "selectedTag") {
            Some((_, slug)) => tag_slug(&slug),
            None => Some(ReaderTagSlug("dailyprompt".to_string())),
        }
        .and_then(|slug| stream(ReaderStream::Discover { tags: vec![slug] })),
        ["read" | "reader", "blogs", id] | ["read" | "reader", "blogs", id, "posts"] => {
            site(id).map(ReaderAddress::Site)
        }
        ["read" | "reader", "feeds", id] | ["read" | "reader", "feeds", id, "posts"] => {
            feed(id).map(ReaderAddress::Feed)
        }
        ["read" | "reader", "blogs", id, "posts", post] => site(id)
            .zip(reader_id(post))
            .map(|(site, post)| ReaderAddress::SitePost(site, post)),
        ["read" | "reader", "feeds", id, "posts", item] => feed(id)
            .zip(reader_id(item).map(ReaderFeedItemId))
            .map(|(feed, item)| ReaderAddress::FeedPost(feed, item)),
        ["read", "blog", "id", id] => site(id).map(ReaderAddress::Site),
        ["read", "blog", "feed", id] => feed(id).map(ReaderAddress::Feed),
        ["read", "post", "id", id, post] => site(id)
            .zip(reader_id(post))
            .map(|(site, post)| ReaderAddress::SitePost(site, post)),
        ["read", "post", "feed", id, item] => feed(id)
            .zip(reader_id(item).map(ReaderFeedItemId))
            .map(|(feed, item)| ReaderAddress::FeedPost(feed, item)),
        _ => None,
    };
    Some(named.unwrap_or(ReaderAddress::Nothing))
}

/// Whether an error is the server's answer that IDs in a Reader address name nothing.
fn names_nothing(error: &ReaderError) -> bool {
    const NOTHING: [&str; 5] = [
        "unknown_blog",
        "site_gone",
        "no_such_feed",
        "unknown_post",
        "no_such_item",
    ];
    matches!(error, ReaderError::Api(error) if NOTHING.iter().any(|code| is_code(error, code)))
}

#[uniffi::export]
impl ReaderService {
    /// Finds what a link opens in the Reader: a post, a site, a feed, or nothing the Reader can
    /// show, which is [`ReaderLink::NotInReader`] with the reason. A post is fetched and kept in
    /// the cache, and a site or feed's details are too. An error is the server refusing, such as
    /// for a private site the reader can't see, or the request failing.
    ///
    /// - One of the Reader's own addresses, such as `https://wordpress.com/read/blogs/{site}` or
    ///   `https://wordpress.com/reader/feeds/{feed}/posts/{item}`, is read for the site, feed or
    ///   post it names, which is then fetched as [`Self::fetch_site`], [`Self::fetch_feed`] or
    ///   [`Self::fetch_post`] fetch it: nothing is sent to the resolver, and a public site's or a
    ///   feed's needs no account. One that names nothing, or names IDs the server doesn't have, is
    ///   [`ReaderLinkNotInReader::NothingThere`]. The module's documentation lists the shapes.
    /// - One of the Reader's own pages of another stream, such as `https://wordpress.com/tag/cats`
    ///   or `https://wordpress.com/discover/search?q=cats`, is a [`ReaderLink::Stream`], and the
    ///   search page with no query a [`ReaderLink::Search`]. Nothing is sent. Signed out, the
    ///   address of a stream that needs an account, such as Following or Liked, is
    ///   [`ReaderError::NeedsAccount`].
    /// - A site's home address, such as `https://example.com/`, is looked up as a WordPress.com
    ///   or Jetpack site, then as an address with a feed. It needs no account.
    /// - Any other address is resolved to a post, which needs an account: the server only does it
    ///   with a token. Where the server finds no post, the address is looked up for a feed, so a
    ///   site's address with a path, such as `https://wordpress.com/blog/`, opens the site.
    /// - Signed out, such an address is tried as a post by its slug instead, as the Android app
    ///   tries it: the address's host is the site, and the last part of its path the slug. A post
    ///   found that way is a [`ReaderLink::Post`]. A host that's no WordPress.com or Jetpack
    ///   site's (404 `unknown_blog`) is looked up for a feed, as it is with an account. Anything
    ///   else is [`ReaderError::NeedsAccount`], since the resolver might find what the slug
    ///   didn't: a server's refusal of the slug, such as no post with it or a private site. An
    ///   address with no slug has its host looked up as a site instead: a feed or nothing when no
    ///   site has it, and [`ReaderError::NeedsAccount`] when one does or the server refuses. A
    ///   request that fails, rather than being refused, is the error. See [`post_slug`] for what
    ///   was measured.
    /// - The server answers an address that isn't on a WordPress.com or Jetpack site with a 404
    ///   `blog_not_found`, and the address is looked up for a feed. Until 2026-11-20 it answered
    ///   with a 500 instead: the site's domain is then looked up, and when no WordPress.com or
    ///   Jetpack site has it, the address is looked up for a feed. When one does, the 500 is the
    ///   error.
    /// - A `wp.me` short link isn't resolved by the server: [`ReaderLinkNotInReader::ShortLink`].
    pub async fn resolve_link(&self, url: String) -> Result<ReaderLink, ReaderError> {
        let not_in_reader = |reason| Ok(ReaderLink::NotInReader { reason });
        let Some(parsed) = parse_link(&url) else {
            return not_in_reader(ReaderLinkNotInReader::NotAnAddress);
        };
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if host == SHORT_LINK_HOST {
            return not_in_reader(ReaderLinkNotInReader::ShortLink);
        }
        if let Some(named) = reader_address(&host, &parsed) {
            return match self.open_reader_address(named).await {
                Err(error) if names_nothing(&error) => {
                    not_in_reader(ReaderLinkNotInReader::NothingThere)
                }
                opened => opened,
            };
        }
        let address = parsed.to_string();

        // Where no post is found, what's left is a feed at the address: no site has one for a
        // page that isn't there.
        let no_feed_reason = if parsed.path().trim_matches('/').is_empty() {
            if let Some(site) = self.site_at(&host).await? {
                return Ok(site);
            }
            ReaderLinkNotInReader::NotWordPressOrFeed
        } else if self.context.owner == ReaderOwner::SignedOut {
            match self.post_by_slug_signed_out(&host, &parsed).await? {
                Some(link) => return Ok(link),
                // No WordPress.com or Jetpack site has the host: a feed or nothing, as signed in.
                None => ReaderLinkNotInReader::NotWordPressOrFeed,
            }
        } else {
            let resolved = self
                .context
                .client
                .reader_post()
                .resolve_url(&ReaderResolveUrlParams {
                    url: address.clone(),
                })
                .await;
            match resolved {
                Ok(resolved) => {
                    let post = self
                        .fetch_post(ReaderPostSaveIdentity {
                            id: resolved.data.post_id.0 as u64,
                            site_id: Some(resolved.data.site_id),
                            feed_id: None,
                            feed_item_id: None,
                            is_external: false,
                            url: Some(address),
                        })
                        .await?;
                    return Ok(ReaderLink::Post { post });
                }
                Err(error) if is_code(&error, "post_not_found") => {
                    ReaderLinkNotInReader::NothingThere
                }
                // The server's answer for an address on any other site since 2026-11-20: no
                // WordPress.com or Jetpack site has the domain, so it's a feed or nothing.
                Err(error) if is_code(&error, "blog_not_found") => {
                    ReaderLinkNotInReader::NotWordPressOrFeed
                }
                Err(error) if error.status_code() == Some(500) => {
                    // The server's answer for an address on any other site. Unless the domain is
                    // a WordPress.com or Jetpack site's, it's a feed or nothing.
                    match self.site_at(&host).await {
                        Ok(None) => ReaderLinkNotInReader::NotWordPressOrFeed,
                        Ok(Some(_)) => return Err(error.into()),
                        Err(lookup) => {
                            log::warn!(
                                "Couldn't look up the domain of a link the server failed on: {lookup}"
                            );
                            return Err(error.into());
                        }
                    }
                }
                Err(error) => return Err(error.into()),
            }
        };
        match self.feed_at(&address).await? {
            Some(found) => Ok(found),
            None => not_in_reader(no_feed_reason),
        }
    }
}

impl ReaderService {
    /// Opens what a Reader address names. A feed of a WordPress.com or Jetpack site opens as the
    /// site, as the web Reader opens it.
    async fn open_reader_address(&self, named: ReaderAddress) -> Result<ReaderLink, ReaderError> {
        Ok(match named {
            ReaderAddress::Site(site_id) => ReaderLink::Site {
                site_id,
                site: self.fetch_site(site_id).await?,
            },
            ReaderAddress::Feed(feed_id) => {
                let site = self.fetch_feed(feed_id).await?;
                match site.site_id.filter(|site_id| site_id.0 != 0) {
                    Some(site_id) => ReaderLink::Site { site_id, site },
                    None => ReaderLink::Feed { feed_id, site },
                }
            }
            ReaderAddress::SitePost(site_id, post_id) => ReaderLink::Post {
                post: self
                    .fetch_post(ReaderPostSaveIdentity {
                        id: post_id,
                        site_id: Some(site_id),
                        feed_id: None,
                        feed_item_id: None,
                        is_external: false,
                        url: None,
                    })
                    .await?,
            },
            ReaderAddress::FeedPost(feed_id, item_id) => ReaderLink::Post {
                post: self
                    .fetch_post(ReaderPostSaveIdentity {
                        id: item_id.0,
                        site_id: None,
                        feed_id: Some(feed_id),
                        feed_item_id: Some(item_id),
                        is_external: true,
                        url: None,
                    })
                    .await?,
            },
            ReaderAddress::Stream(stream) => {
                if stream.needs_account() && self.context.owner == ReaderOwner::SignedOut {
                    return Err(ReaderError::NeedsAccount);
                }
                ReaderLink::Stream { stream }
            }
            ReaderAddress::SearchPage => ReaderLink::Search,
            ReaderAddress::Nothing => ReaderLink::NotInReader {
                reason: ReaderLinkNotInReader::NothingThere,
            },
        })
    }

    /// Signed out, the post at an address by its slug, on the site of its host. `None` when no
    /// WordPress.com or Jetpack site has the host, so the resolver couldn't find a post there
    /// either. Anything else the server refuses is [`ReaderError::NeedsAccount`]: the resolver,
    /// which needs an account, might still find it. An address with no slug has its host looked
    /// up instead, and is [`ReaderError::NeedsAccount`] when a site has it. See
    /// [`ReaderService::resolve_link`].
    async fn post_by_slug_signed_out(
        &self,
        host: &str,
        url: &url::Url,
    ) -> Result<Option<ReaderLink>, ReaderError> {
        let Some(slug) = post_slug(url) else {
            return match self.site_at(host).await {
                Ok(None) => Ok(None),
                Ok(Some(_)) => Err(ReaderError::NeedsAccount),
                Err(error) if is_refused(&error) => Err(ReaderError::NeedsAccount),
                Err(error) => Err(error),
            };
        };
        let found = self
            .context
            .client
            .reader_post()
            .post_by_slug(
                &WpComSiteIdentifier::Slug {
                    value: host.to_string(),
                },
                &ReaderPostSlug(slug),
                &ReaderPostBySlugParams::default(),
            )
            .await;
        let found = match found {
            Ok(found) => found.data,
            // Measured on 2026-11-20 for 19 hosts: only a host that's no WordPress.com or Jetpack
            // site's answers this. Another form of a site's domain, such as with `www.`, finds
            // the site.
            Err(error) if is_code(&error, "unknown_blog") => return Ok(None),
            Err(error) => {
                let error = ReaderError::from(error);
                return Err(if is_refused(&error) {
                    ReaderError::NeedsAccount
                } else {
                    error
                });
            }
        };
        let post = self
            .fetch_post(ReaderPostSaveIdentity {
                id: found.post_id.0 as u64,
                site_id: Some(found.site_id),
                feed_id: None,
                feed_item_id: None,
                is_external: false,
                url: found.url.or(Some(url.to_string())),
            })
            .await?;
        Ok(Some(ReaderLink::Post { post }))
    }

    /// The WordPress.com or Jetpack site with a domain, now in the cache, or `None` when no site
    /// has it.
    async fn site_at(&self, domain: &str) -> Result<Option<ReaderLink>, ReaderError> {
        let site = self
            .context
            .client
            .reader_sites()
            .site(
                &WpComSiteIdentifier::Slug {
                    value: domain.to_string(),
                },
                &ReaderSiteParams::default(),
            )
            .await;
        match site {
            Ok(site) => {
                let site_id = site.data.site_id;
                let site = self
                    .context
                    .store_site(super::sites::site_record(&site.data))?;
                Ok(Some(ReaderLink::Site { site_id, site }))
            }
            Err(error) if is_code(&error, "unknown_blog") => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// The first feed the server finds at an address, now in the cache: the site's stream when it
    /// belongs to a WordPress.com or Jetpack site, and the feed's when it doesn't. `None` when it
    /// finds none.
    async fn feed_at(&self, address: &str) -> Result<Option<ReaderLink>, ReaderError> {
        let params = ReaderFeedSearchParams {
            url: Some(address.to_string()),
            meta: Some("feed".to_string()),
            ..Default::default()
        };
        let found = match self
            .context
            .client
            .reader_sites()
            .search_feeds(&params)
            .await
        {
            Ok(found) => found.data,
            Err(error) if is_code(&error, "no_such_feed") => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let Some(result) = found.feeds.into_iter().next() else {
            return Ok(None);
        };
        let Some(feed_id) = result
            .feed_id
            .or(result.feed.as_ref().map(|feed| feed.feed_id))
        else {
            return Ok(None);
        };
        let site_id = result
            .feed
            .as_ref()
            .and_then(|feed| feed.site_id)
            .filter(|site| site.0 != 0);
        let record = match &result.feed {
            Some(feed) => super::sites::feed_record(feed),
            None => ReaderSiteRecord {
                site_id,
                feed_id: Some(feed_id),
                name: None,
                description: None,
                url: result.subscribe_url.clone(),
                feed_url: result.subscribe_url,
                icon_url: None,
                subscribers_count: None,
                post_count: None,
                is_private: None,
                is_jetpack: None,
                organization_id: None,
                is_following: None,
            },
        };
        let site = self.context.store_site(record)?;
        Ok(Some(match site_id {
            Some(site_id) => ReaderLink::Site { site_id, site },
            None => ReaderLink::Feed { feed_id, site },
        }))
    }
}
