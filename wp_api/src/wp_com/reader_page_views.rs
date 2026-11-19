//! Counting a view of a post read in the Reader.
//!
//! Fetching a post through the API counts nothing. A site's stats count a view in the Reader only
//! when the app asks for WordPress.com's tracking pixel, which is what [`ReaderPageView::request`]
//! builds. It isn't an API request: it goes to another host, and carries no authentication.

use crate::{
    posts::PostId,
    request::{RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, endpoint::WpEndpointUrl},
    wp_com::WpComSiteId,
};
use http::{HeaderMap, HeaderValue, header::REFERER};
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
}

impl ReaderPageView {
    /// The request that counts the view, or `None` if [`Self::site_url`] has no host.
    pub fn request(&self) -> Option<WpNetworkRequest> {
        // A number that differs each time, so the request is never answered from a cache.
        let nonce = Uuid::new_v4().as_u128() as u32;
        self.request_with_nonce(nonce)
    }

    fn request_with_nonce(&self, nonce: u32) -> Option<WpNetworkRequest> {
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
/// host. Send it with the app's request executor.
#[uniffi::export]
pub fn reader_page_view_request(view: ReaderPageView) -> Option<Arc<WpNetworkRequest>> {
    view.request().map(Arc::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::NetworkRequestAccessor;

    fn view() -> ReaderPageView {
        ReaderPageView {
            site_id: WpComSiteId(123),
            post_id: PostId(456),
            site_url: "https://example.wordpress.com/a/path".to_string(),
            site_is_private: false,
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
