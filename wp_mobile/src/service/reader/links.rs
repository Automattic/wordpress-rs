//! Links: what a post's, a site's or a feed's address opens in the Reader.
//!
//! The server has no one request for this. What each one answers, as found live:
//!
//! - `GET /wpcom/v2/mobile/resolve-reader-url` resolves a post's address on a WordPress.com or
//!   Jetpack site to its IDs, on the site's own domain too, with a query or fragment or without.
//!   It needs a token: signed out, every address is a 401 `rest_forbidden`. A site's home address
//!   is a 400, whether signed in or not: the API wants a path. Any other address on a WordPress.com
//!   or Jetpack site, such as a page that isn't there, a category, a feed or a `wp.me` short link,
//!   is a 404 `post_not_found`. An address on any other site is a 500 with no body.
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
//! 404 `no_such_item`. The Reader's other pages, such as a tag's, a list's, search or Discover,
//! open a stream that isn't a site or a feed, and `ReaderLink` has nothing for them yet.

use super::actions::is_refused;
use super::{ReaderError, ReaderService};
use wp_api::prelude::{WpApiError, WpErrorCode};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_post::{ReaderPostBySlugParams, ReaderPostSlug, ReaderResolveUrlParams};
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
    /// names no site, feed or post: one the Reader doesn't have, or one of its other pages.
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
            "There's no post, site or feed at that address for the Reader to show."
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReaderAddress {
    Site(WpComSiteId),
    Feed(ReaderFeedId),
    SitePost(WpComSiteId, u64),
    FeedPost(ReaderFeedId, ReaderFeedItemId),
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

/// What a Reader address names, or `None` when the address isn't one of the Reader's: on
/// another host, or with a path that doesn't start with `read` or `reader`. `host` is in lower
/// case.
fn reader_address(host: &str, url: &url::Url) -> Option<ReaderAddress> {
    if !READER_HOSTS.contains(&host) {
        return None;
    }
    let segments: Vec<&str> = url
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect();
    let (first, rest) = segments.split_first()?;
    let is_read = first.eq_ignore_ascii_case("read");
    if !is_read && !first.eq_ignore_ascii_case("reader") {
        return None;
    }
    // The web matches the words of a Reader address in any case, and the IDs as digits.
    let words: Vec<String> = rest.iter().map(|word| word.to_ascii_lowercase()).collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let site = |id: &str| reader_id(id).map(WpComSiteId);
    let feed = |id: &str| reader_id(id).map(ReaderFeedId);
    let named = match words.as_slice() {
        ["blogs", id] | ["blogs", id, "posts"] => site(id).map(ReaderAddress::Site),
        ["feeds", id] | ["feeds", id, "posts"] => feed(id).map(ReaderAddress::Feed),
        ["blogs", id, "posts", post] => site(id)
            .zip(reader_id(post))
            .map(|(site, post)| ReaderAddress::SitePost(site, post)),
        ["feeds", id, "posts", item] => feed(id)
            .zip(reader_id(item).map(ReaderFeedItemId))
            .map(|(feed, item)| ReaderAddress::FeedPost(feed, item)),
        ["blog", "id", id] if is_read => site(id).map(ReaderAddress::Site),
        ["blog", "feed", id] if is_read => feed(id).map(ReaderAddress::Feed),
        ["post", "id", id, post] if is_read => site(id)
            .zip(reader_id(post))
            .map(|(site, post)| ReaderAddress::SitePost(site, post)),
        ["post", "feed", id, item] if is_read => feed(id)
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
    /// - A site's home address, such as `https://example.com/`, is looked up as a WordPress.com
    ///   or Jetpack site, then as an address with a feed. It needs no account.
    /// - Any other address is resolved to a post, which needs an account: the server only does it
    ///   with a token. Where the server finds no post, the address is looked up for a feed, so a
    ///   site's address with a path, such as `https://wordpress.com/blog/`, opens the site.
    /// - Signed out, such an address is tried as a post by its slug instead, as the Android app
    ///   tries it: the address's host is the site, and the last part of its path the slug. A post
    ///   found that way is a [`ReaderLink::Post`]. Anything else is
    ///   [`ReaderError::NeedsAccount`], since the resolver might find what the slug didn't: a
    ///   server's refusal of the slug, such as no post with it, a host that's no site's, or a
    ///   private site, and an address with no slug, for which nothing is sent. A request that
    ///   fails, rather than being refused, is the error. See [`post_slug`] for what was measured.
    /// - The server answers an address that isn't on a WordPress.com or Jetpack site with a 500.
    ///   The site's domain is then looked up, and when no WordPress.com or Jetpack site has it,
    ///   the address is looked up for a feed. When one does, the 500 is the error.
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
        } else {
            if self.context.owner == ReaderOwner::SignedOut {
                return self.post_by_slug_signed_out(&host, &parsed).await;
            }
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
            ReaderAddress::Nothing => ReaderLink::NotInReader {
                reason: ReaderLinkNotInReader::NothingThere,
            },
        })
    }

    /// Signed out, the post at an address by its slug, on the site of its host. Anything the server
    /// refuses, and an address with no slug, is [`ReaderError::NeedsAccount`]: the resolver, which
    /// needs an account, might still find it. See [`ReaderService::resolve_link`].
    async fn post_by_slug_signed_out(
        &self,
        host: &str,
        url: &url::Url,
    ) -> Result<ReaderLink, ReaderError> {
        let Some(slug) = post_slug(url) else {
            return Err(ReaderError::NeedsAccount);
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
            .await
            .map_err(ReaderError::from);
        let found = match found {
            Ok(found) => found.data,
            Err(error) if is_refused(&error) => return Err(ReaderError::NeedsAccount),
            Err(error) => return Err(error),
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
        Ok(ReaderLink::Post { post })
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
