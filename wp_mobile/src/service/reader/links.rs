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

use super::{ReaderError, ReaderService};
use wp_api::prelude::{WpApiError, WpErrorCode};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_post::ReaderResolveUrlParams;
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderPostSaveIdentity};
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
    /// feed, such as a page that isn't there.
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

#[uniffi::export]
impl ReaderService {
    /// Finds what a link opens in the Reader: a post, a site, a feed, or nothing the Reader can
    /// show, which is [`ReaderLink::NotInReader`] with the reason. A post is fetched and kept in
    /// the cache, and a site or feed's details are too. An error is the server refusing, such as
    /// for a private site the reader can't see, or the request failing.
    ///
    /// - A site's home address, such as `https://example.com/`, is looked up as a WordPress.com
    ///   or Jetpack site, then as an address with a feed. It needs no account.
    /// - Any other address is resolved to a post, which needs an account: the server only does it
    ///   with a token, so signed out it's [`ReaderError::NeedsAccount`], and nothing is sent.
    ///   Where the server finds no post, the address is looked up for a feed, so a site's address
    ///   with a path, such as `https://wordpress.com/blog/`, opens the site.
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
                return Err(ReaderError::NeedsAccount);
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
