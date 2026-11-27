//! Fetching an image of a Reader post to keep on the device, so the post can be read with no
//! connection.
//!
//! It isn't an API request: an image is wherever its post's site put it. Only an image of a
//! private WordPress.com site needs the account's authorization, and only WordPress.com gets it.

use crate::{
    auth::WpAuthenticationProvider,
    request::{
        RequestMethod, WpNetworkHeaderMap, WpNetworkRequest, WpNetworkResponse,
        endpoint::WpEndpointUrl,
    },
};
use http::{HeaderMap, HeaderValue, header::ACCEPT};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

/// The request that fetches the image at `url`, or `None` if that isn't an `http` or `https`
/// address.
///
/// `authentication` is the account's, for an image of a private site. It's sent only over
/// `https`, and only to `wordpress.com` and its subdomains, which is where a private
/// WordPress.com site's images are: any other host gets the request without it.
pub fn reader_image_request(
    url: &str,
    authentication: Option<&WpAuthenticationProvider>,
) -> Option<WpNetworkRequest> {
    let url = Url::parse(url).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("image/*"));
    if let Some(authentication) = authentication
        && url.scheme() == "https"
        && url.host_str().is_some_and(is_wordpress_com)
    {
        authentication.insert_header(&mut headers);
    }
    Some(WpNetworkRequest {
        uuid: Uuid::new_v4().into(),
        retry_count: 0,
        method: RequestMethod::GET,
        url: WpEndpointUrl(url.to_string()),
        header_map: Arc::new(WpNetworkHeaderMap::new(headers)),
        body: None,
    })
}

fn is_wordpress_com(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "wordpress.com" || host.ends_with(".wordpress.com")
}

/// The extension to store a fetched image under, or `None` if the answer isn't an image: a site
/// that answers a missing image with a page, for example.
///
/// It goes by the answer's `Content-Type`. An answer without one is taken at the word of the
/// address it was fetched from.
pub fn reader_image_file_extension(response: &WpNetworkResponse) -> Option<&'static str> {
    match response.response_header_map.get_str("content-type") {
        Some(content_type) => {
            let content_type = content_type
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            IMAGE_TYPES
                .iter()
                .find(|(image_type, _)| *image_type == content_type)
                .map(|(_, extension)| *extension)
        }
        None => {
            let url = Url::parse(&response.request_url.0).ok()?;
            let extension = url.path().rsplit_once('.')?.1.to_ascii_lowercase();
            IMAGE_TYPES
                .iter()
                .map(|(_, known)| *known)
                .chain(["jpeg"])
                .find(|known| *known == extension)
                .map(|known| if known == "jpeg" { "jpg" } else { known })
        }
    }
}

/// The image types a web view shows, and the extension each is stored under.
const IMAGE_TYPES: [(&str, &str); 10] = [
    ("image/jpeg", "jpg"),
    ("image/png", "png"),
    ("image/gif", "gif"),
    ("image/webp", "webp"),
    ("image/avif", "avif"),
    ("image/svg+xml", "svg"),
    ("image/bmp", "bmp"),
    ("image/x-icon", "ico"),
    ("image/vnd.microsoft.icon", "ico"),
    ("image/heic", "heic"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::WpAuthentication;
    use rstest::rstest;

    fn bearer() -> WpAuthenticationProvider {
        WpAuthenticationProvider::static_with_auth(WpAuthentication::Bearer {
            token: "secret".to_string(),
        })
    }

    fn authorization(request: &WpNetworkRequest) -> Option<String> {
        request
            .header_map
            .get_str("authorization")
            .map(str::to_string)
    }

    #[test]
    fn test_request_asks_for_an_image_without_authorization() {
        let request = reader_image_request("https://example.com/a.jpg?w=100", None).unwrap();

        assert_eq!(request.method, RequestMethod::GET);
        assert_eq!(request.url.0, "https://example.com/a.jpg?w=100");
        assert_eq!(request.header_map.get_str("accept"), Some("image/*"));
        assert_eq!(authorization(&request), None);
        assert!(request.body.is_none());
    }

    #[rstest]
    #[case("https://wordpress.com/a.jpg")]
    #[case("https://example.files.wordpress.com/2026/11/a.jpg")]
    #[case("https://EXAMPLE.WordPress.com/a.jpg")]
    fn test_request_sends_authorization_to_wordpress_com(#[case] url: &str) {
        let request = reader_image_request(url, Some(&bearer())).unwrap();
        assert_eq!(authorization(&request).as_deref(), Some("Bearer secret"));
    }

    /// The account's token goes nowhere else, whatever a post's content says.
    #[rstest]
    #[case::another_host("https://example.com/a.jpg")]
    #[case::a_host_that_ends_like_it("https://notwordpress.com/a.jpg")]
    #[case::a_host_that_starts_like_it("https://wordpress.com.example.com/a.jpg")]
    #[case::without_tls("http://example.files.wordpress.com/a.jpg")]
    #[case::credentials_in_the_address("https://wordpress.com@example.com/a.jpg")]
    fn test_request_keeps_authorization_from_other_hosts(#[case] url: &str) {
        let request = reader_image_request(url, Some(&bearer())).unwrap();
        assert_eq!(authorization(&request), None);
    }

    #[rstest]
    #[case("data:image/png;base64,AAAA")]
    #[case("file:///etc/passwd")]
    #[case("/images/a.png")]
    #[case("")]
    fn test_no_request_for_what_is_not_a_web_address(#[case] url: &str) {
        assert!(reader_image_request(url, Some(&bearer())).is_none());
    }

    fn response(url: &str, content_type: Option<&str>) -> WpNetworkResponse {
        let mut headers = HeaderMap::new();
        if let Some(content_type) = content_type {
            headers.insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_str(content_type).unwrap(),
            );
        }
        WpNetworkResponse {
            body: vec![1],
            status_code: 200,
            response_header_map: Arc::new(WpNetworkHeaderMap::new(headers)),
            request_url: WpEndpointUrl(url.to_string()),
            request_method: RequestMethod::GET,
            request_header_map: Arc::new(WpNetworkHeaderMap::default()),
        }
    }

    #[rstest]
    #[case("https://example.com/a", Some("image/jpeg"), Some("jpg"))]
    #[case(
        "https://example.com/a.png",
        Some("IMAGE/WEBP; charset=binary"),
        Some("webp")
    )]
    #[case("https://example.com/a.svg", Some("image/svg+xml"), Some("svg"))]
    #[case::a_page_instead_of_an_image("https://example.com/a.jpg", Some("text/html"), None)]
    #[case::no_type_and_a_known_extension("https://example.com/a.JPEG?w=1", None, Some("jpg"))]
    #[case::no_type_and_no_extension("https://example.com/image", None, None)]
    #[case::no_type_and_another_extension("https://example.com/a.php", None, None)]
    fn test_file_extension(
        #[case] url: &str,
        #[case] content_type: Option<&str>,
        #[case] expected: Option<&str>,
    ) {
        assert_eq!(
            reader_image_file_extension(&response(url, content_type)),
            expected
        );
    }
}
