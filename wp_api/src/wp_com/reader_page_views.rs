//! Counting a view of a post read in the Reader.
//!
//! Fetching a post through the API counts nothing. A site's stats count a view in the Reader only
//! when the app asks for WordPress.com's tracking pixel, which is what [`ReaderPageView::request`]
//! builds. It isn't an API request: it goes to another host, carries no authentication, and is
//! sent with the app's `User-Agent` in place of the library's.

use crate::{
    posts::PostId,
    request::{RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, endpoint::WpEndpointUrl},
    wp_com::WpComSiteId,
};
use http::{
    HeaderMap, HeaderValue,
    header::{REFERER, USER_AGENT},
};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

/// The page a view in the Reader is counted as coming from. The pixel doesn't count a view sent
/// without it as the `Referer`.
pub const READER_PAGE_VIEW_REFERRER: &str = "https://wordpress.com/";

const PIXEL_URL: &str = "https://pixel.wp.com/g.gif";

/// A view of a post in the Reader, to count in its site's stats.
///
/// Only a post of a WordPress.com or Jetpack site has stats to count it in. An RSS post has none.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReaderPageView {
    pub site_id: WpComSiteId,
    pub post_id: PostId,
    /// The site's URL. Its host is sent with the view.
    pub site_url: String,
    /// Whether the site is private. The view is then marked as one by a reader allowed to see it.
    pub site_is_private: bool,
    /// The `User-Agent` the view is sent with: the one the app's web view sends, which the
    /// WordPress apps follow with their own name, as in `… Mobile/15E148 wp-iphone/26.4`.
    ///
    /// It can't be the library's own, or blank: see [`reader_page_view_user_agent_counts`].
    pub user_agent: String,
}

/// Whether a site's stats count a view sent with this `User-Agent`: it isn't blank, it can be
/// sent as a header, and it doesn't have `wordpress` in it.
///
/// Stats drop a view whose `User-Agent` has `wordpress` in it, which is how a WordPress site's
/// own requests name themselves, and how this library names itself in every API request. The
/// pixel answers such a view with 200 all the same, so nothing but the site's stats shows that it
/// wasn't counted. Seen on 2026-11-19: views sent with `wordpress-rs/…` at the start or the end
/// of the `User-Agent` never showed in the site's stats, and the same views sent as `wp-rs/…`,
/// or with the `User-Agent` of a web view or of `URLSession`, showed within a minute.
#[uniffi::export]
pub fn reader_page_view_user_agent_counts(user_agent: &str) -> bool {
    user_agent_header(user_agent).is_some()
}

fn user_agent_header(user_agent: &str) -> Option<HeaderValue> {
    let user_agent = user_agent.trim();
    if user_agent.is_empty() || user_agent.to_ascii_lowercase().contains("wordpress") {
        return None;
    }
    HeaderValue::from_str(user_agent).ok()
}

impl ReaderPageView {
    /// The request that counts the view, or `None` if [`Self::site_url`] has no host or
    /// [`Self::user_agent`] is one a view isn't counted with.
    pub fn request(&self) -> Option<WpNetworkRequest> {
        // A number that differs each time, so the request is never answered from a cache.
        let nonce = Uuid::new_v4().as_u128() as u32;
        self.request_with_nonce(nonce)
    }

    fn request_with_nonce(&self, nonce: u32) -> Option<WpNetworkRequest> {
        let user_agent = user_agent_header(&self.user_agent)?;
        let host = Url::parse(&self.site_url)
            .ok()?
            .host_str()
            .map(str::to_string)?;
        let mut url = Url::parse(PIXEL_URL).expect("the pixel's URL parses");
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("v", "wpcom")
                .append_pair("reader", "1")
                .append_pair("blog", &self.site_id.0.to_string())
                .append_pair("post", &self.post_id.0.to_string())
                .append_pair("host", &host)
                .append_pair("ref", READER_PAGE_VIEW_REFERRER)
                .append_pair("t", &nonce.to_string());
            if self.site_is_private {
                query.append_pair("priv", "1");
            }
        }
        let mut headers = HeaderMap::new();
        headers.insert(REFERER, HeaderValue::from_static(READER_PAGE_VIEW_REFERRER));
        // The executor sends this in place of the library's own `User-Agent`.
        headers.insert(USER_AGENT, user_agent);
        Some(WpNetworkRequest {
            uuid: Uuid::new_v4().into(),
            retry_count: 0,
            method: RequestMethod::GET,
            url: WpEndpointUrl(url.to_string()),
            header_map: Arc::new(WpNetworkHeaderMap::new(headers)),
            body: None,
        })
    }
}

/// The request that counts a view of a post in the Reader, or `None` if the site's URL has no
/// host or the view's `User-Agent` is one a view isn't counted with. Send it with the app's
/// request executor.
#[uniffi::export]
pub fn reader_page_view_request(view: ReaderPageView) -> Option<Arc<WpNetworkRequest>> {
    view.request().map(Arc::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::NetworkRequestAccessor;

    const USER_AGENT_OF_AN_APP: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 wp-iphone/26.4";

    fn view() -> ReaderPageView {
        ReaderPageView {
            site_id: WpComSiteId(123),
            post_id: PostId(456),
            site_url: "https://example.wordpress.com/a/path".to_string(),
            site_is_private: false,
            user_agent: USER_AGENT_OF_AN_APP.to_string(),
        }
    }

    #[test]
    fn test_the_request_for_a_view() {
        let request = view().request_with_nonce(7).unwrap();
        assert_eq!(request.method(), RequestMethod::GET);
        assert_eq!(
            request.url().0,
            "https://pixel.wp.com/g.gif?v=wpcom&reader=1&blog=123&post=456&host=example.wordpress.com&ref=https%3A%2F%2Fwordpress.com%2F&t=7"
        );
        let headers = request.header_map().to_header_map();
        assert_eq!(headers.get(REFERER).unwrap(), "https://wordpress.com/");
        assert_eq!(headers.get(USER_AGENT).unwrap(), USER_AGENT_OF_AN_APP);
        // The pixel isn't the API, so the request carries no authentication.
        assert!(!request.has_http_authentication());
    }

    #[test]
    fn test_a_view_of_a_private_site_says_so() {
        let request = ReaderPageView {
            site_is_private: true,
            ..view()
        }
        .request_with_nonce(7)
        .unwrap();
        assert!(request.url().0.ends_with("&t=7&priv=1"));
    }

    #[test]
    fn test_two_requests_for_a_view_differ() {
        assert_ne!(
            view().request().unwrap().url().0,
            view().request().unwrap().url().0
        );
    }

    #[test]
    fn test_a_view_isnt_sent_with_a_user_agent_stats_dont_count() {
        let library = crate::request::user_agent::default_user_agent("URLSession");
        for user_agent in [
            "",
            "   ",
            library.as_str(),
            "WordPress/6.8; https://example.com",
            "Example/1.0 CFNetwork/3826.500.111 Darwin/27.2.0 wordpress-rs/0.11.1",
            "A line\nbreak can't be sent in a header",
        ] {
            assert!(
                !reader_page_view_user_agent_counts(user_agent),
                "{user_agent:?}"
            );
            let view = ReaderPageView {
                user_agent: user_agent.to_string(),
                ..view()
            };
            assert!(view.request().is_none(), "{user_agent:?}");
        }
    }

    #[test]
    fn test_a_view_is_sent_with_an_apps_user_agent() {
        for user_agent in [
            USER_AGENT_OF_AN_APP,
            "Example/1.0 CFNetwork/3826.500.111 Darwin/27.2.0",
            "  wp-rs-example/1.0  ",
        ] {
            assert!(
                reader_page_view_user_agent_counts(user_agent),
                "{user_agent:?}"
            );
            let view = ReaderPageView {
                user_agent: user_agent.to_string(),
                ..view()
            };
            let request = view.request().unwrap();
            let headers = request.header_map().to_header_map();
            assert_eq!(headers.get(USER_AGENT).unwrap(), user_agent.trim());
        }
    }

    #[test]
    fn test_a_site_url_without_a_host_has_no_request() {
        for site_url in ["", "not a url", "mailto:someone@example.com"] {
            let view = ReaderPageView {
                site_url: site_url.to_string(),
                ..view()
            };
            assert!(view.request().is_none(), "{site_url}");
        }
    }
}
