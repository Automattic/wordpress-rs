//! Recognises error responses that Cloudflare generated itself, rather than
//! passed through from the site's server.
//!
//! Every response from a site behind Cloudflare carries `server: cloudflare` and
//! a `cf-ray` header, including the site's own successes and errors. So the
//! headers only say the request went through Cloudflare; the body (or the
//! `cf-mitigated` header) is what says Cloudflare produced the response.
//!
//! Cloudflare sends its own errors in one of these shapes:
//!
//! - RFC 9457 problem JSON with `"cloudflare_error": true`, for 1xxx and 5xx
//!   errors when the request asks for JSON. Cloudflare only sends it when the
//!   request also carries an `Accept-Encoding` header, which every platform
//!   executor does.
//! - A plain-text `error code: NNNN` body, when the request asks for JSON
//!   without `Accept-Encoding`.
//! - Its branded HTML error page, when the request accepts HTML or sends no
//!   `Accept` header (as the login flow does). 5xx pages say `Error code 522`;
//!   1xxx pages carry `errorCode: 1016` in a script. A WAF block page ("Sorry,
//!   you have been blocked") has no code.
//! - An nginx-style HTML page signed `<center>cloudflare</center>`, which
//!   carries no code, whatever the request asks for. Seen for 413 and 502.
//! - A bot-protection challenge page, marked by `cf-mitigated: challenge`.
//!
//! Recorded examples of each live in `test-data/cloudflare-responses`.

use crate::request::WpNetworkResponse;
use serde::Deserialize;

const HEADER_CF_RAY: &str = "cf-ray";
const HEADER_CF_MITIGATED: &str = "cf-mitigated";

/// What went wrong, as far as Cloudflare reports it.
///
/// Most kinds are problems between Cloudflare and the site's server, which
/// only the site owner or host can fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CloudflareErrorKind {
    /// The request body is larger than the site's Cloudflare plan allows (413).
    RequestTooLarge,
    /// The site's server sent Cloudflare an invalid response, or none in time
    /// (502, 504, 520).
    OriginError,
    /// The site's server refused Cloudflare's connection (521).
    OriginDown,
    /// The site's server didn't respond in time (522, 524).
    OriginTimeout,
    /// Cloudflare couldn't reach the site's server at all (523), or the site's
    /// DNS doesn't lead to one it can use (1016, 1002).
    OriginUnreachable,
    /// The site's server has an SSL problem (525, 526).
    OriginSsl,
    /// Cloudflare itself failed (500).
    CloudflareInternal,
    /// Cloudflare is rate-limiting requests to the site (1015).
    RateLimited,
    /// A Cloudflare security rule on the site blocked the request (1020, or a
    /// WAF block page).
    AccessDenied,
    /// A bot-protection challenge that an app can never pass.
    Challenge,
    /// Any other Cloudflare error.
    Other,
}

/// A response Cloudflare generated itself, with what could be read from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CloudflareErrorResponse {
    pub cloudflare_code: Option<u32>,
    pub kind: CloudflareErrorKind,
    pub ray_id: Option<String>,
    pub retry_after: Option<u64>,
}

/// The fields of Cloudflare's RFC 9457 error body this crate reads. See
/// <https://developers.cloudflare.com/fundamentals/reference/error-responses/>.
#[derive(Debug, Deserialize)]
struct CloudflareProblemDetails {
    cloudflare_error: Option<bool>,
    error_code: Option<u32>,
    error_category: Option<String>,
    ray_id: Option<String>,
    retry_after: Option<u64>,
}

impl CloudflareErrorResponse {
    /// Returns `Some` only for error responses Cloudflare generated itself.
    ///
    /// Callers must rule out a WordPress error body first: that is the site's
    /// own response, whatever headers it came with.
    pub(crate) fn try_from_response(response: &WpNetworkResponse) -> Option<Self> {
        if response.status_code < 400 || !is_proxied_by_cloudflare(response) {
            return None;
        }
        let ray_id_from_header = response
            .response_header_map
            .get_str(HEADER_CF_RAY)
            .map(ray_id_without_data_center);
        let retry_after = response.get_retry_after();

        if response
            .response_header_map
            .get_str(HEADER_CF_MITIGATED)
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("challenge"))
        {
            return Some(Self {
                cloudflare_code: None,
                kind: CloudflareErrorKind::Challenge,
                ray_id: ray_id_from_header,
                retry_after,
            });
        }

        if let Some(details) = parse_problem_details(&response.body) {
            return Some(Self {
                cloudflare_code: details.error_code,
                kind: kind_for(
                    details.error_code,
                    response.status_code,
                    details.error_category.as_deref(),
                ),
                ray_id: details.ray_id.or(ray_id_from_header),
                retry_after: retry_after.or(details.retry_after),
            });
        }

        let body = String::from_utf8_lossy(&response.body);
        let cloudflare_code = if let Some(code) = parse_plain_text_error_code(&body) {
            Some(code)
        } else if is_branded_error_page(&body) {
            parse_branded_error_page_code(&body)
        } else if is_nginx_style_page(&body) {
            None
        } else {
            return None;
        };
        Some(Self {
            cloudflare_code,
            kind: kind_for(cloudflare_code, response.status_code, None),
            ray_id: ray_id_from_header,
            retry_after,
        })
    }
}

fn is_proxied_by_cloudflare(response: &WpNetworkResponse) -> bool {
    let headers = &response.response_header_map;
    headers
        .get_str(http::header::SERVER.as_str())
        .is_some_and(|server| server.trim().eq_ignore_ascii_case("cloudflare"))
        && headers.get_str(HEADER_CF_RAY).is_some()
}

/// The `cf-ray` header ends in the data center that served the request
/// (`a50c71565ed1595c-SEA`). Cloudflare's error pages and JSON body show the
/// ID without it, so drop it to give callers one format.
fn ray_id_without_data_center(header_value: &str) -> String {
    let header_value = header_value.trim();
    header_value
        .split_once('-')
        .map_or(header_value, |(id, _)| id)
        .to_string()
}

fn parse_problem_details(body: &[u8]) -> Option<CloudflareProblemDetails> {
    let details: CloudflareProblemDetails = serde_json::from_slice(body).ok()?;
    let is_cloudflare = details.cloudflare_error == Some(true)
        || (details.error_code.is_some() && details.ray_id.is_some());
    is_cloudflare.then_some(details)
}

/// Cloudflare's plain-text error body is exactly `error code: NNNN`.
fn parse_plain_text_error_code(body: &str) -> Option<u32> {
    let code = body.trim().strip_prefix("error code: ")?;
    if code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    code.parse().ok()
}

/// The page Cloudflare's edge serves for e.g. 413 and 502.
fn is_nginx_style_page(body: &str) -> bool {
    body.contains("<center>cloudflare</center>")
}

/// Cloudflare's branded error page, for 5xx and 1xxx errors and WAF blocks.
fn is_branded_error_page(body: &str) -> bool {
    body.contains("id=\"cf-error-details\"")
}

/// The code on a branded error page: `Error code 522` on 5xx pages, and
/// `errorCode: 1016` in a script on 1xxx pages. WAF block pages have none.
fn parse_branded_error_page_code(body: &str) -> Option<u32> {
    ["Error code ", "errorCode:"].iter().find_map(|marker| {
        let after = &body[body.find(marker)? + marker.len()..];
        let digits = after.trim_start();
        let end = digits
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(digits.len());
        digits[..end].parse().ok()
    })
}

fn kind_for(code: Option<u32>, status_code: u32, category: Option<&str>) -> CloudflareErrorKind {
    use CloudflareErrorKind::*;
    match code.unwrap_or(status_code) {
        413 => RequestTooLarge,
        502 | 504 | 520 => OriginError,
        521 => OriginDown,
        522 | 524 => OriginTimeout,
        // 1002 and 1016 also arrive as HTML pages, which have no `error_category`.
        523 | 1002 | 1016 => OriginUnreachable,
        525 | 526 => OriginSsl,
        500 => CloudflareInternal,
        1015 => RateLimited,
        1020 => AccessDenied,
        _ => match category {
            Some("access_denied") => AccessDenied,
            Some("rate_limit") => RateLimited,
            // The site's DNS doesn't lead Cloudflare to a server it can use, e.g. 1002.
            Some("dns") => OriginUnreachable,
            // A page without a code is reported by its HTTP status alone.
            _ if code.is_none() && status_code == 403 => AccessDenied,
            _ if code.is_none() && status_code == 429 => RateLimited,
            _ => Other,
        },
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::request::{RequestMethod, WpNetworkHeaderMap, endpoint::WpEndpointUrl};
    use http::{HeaderMap, HeaderName, HeaderValue};
    use rstest::rstest;
    use std::{path::PathBuf, sync::Arc};

    /// Reads a raw HTTP/1.1 response recorded in `test-data/cloudflare-responses`
    /// as if it had come back from a request to `request_url`.
    pub(crate) fn recorded_response(file_name: &str, request_url: &str) -> WpNetworkResponse {
        let path: PathBuf = [
            env!("CARGO_MANIFEST_DIR"),
            "..",
            "test-data",
            "cloudflare-responses",
            file_name,
        ]
        .iter()
        .collect();
        let raw = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("Couldn't read the fixture at {path:?}: {e}"));
        let separator = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("A recorded response separates its head from its body with a blank line");
        let head = std::str::from_utf8(&raw[..separator]).expect("The head is ASCII");
        let body = raw[separator + 4..].to_vec();

        let mut lines = head.split("\r\n");
        let status_code = lines
            .next()
            .and_then(|status_line| status_line.split(' ').nth(1))
            .and_then(|code| code.parse().ok())
            .expect("The status line names a status code");
        let mut headers = HeaderMap::new();
        for line in lines {
            let (name, value) = line.split_once(':').expect("A header line has a colon");
            headers.append(
                HeaderName::from_bytes(name.trim().as_bytes()).unwrap(),
                HeaderValue::from_str(value.trim()).unwrap(),
            );
        }

        WpNetworkResponse {
            body,
            status_code,
            response_header_map: Arc::new(headers.into()),
            request_url: WpEndpointUrl(request_url.to_string()),
            request_method: RequestMethod::GET,
            request_header_map: Arc::new(WpNetworkHeaderMap::default()),
        }
    }

    fn parse(file_name: &str) -> Option<CloudflareErrorResponse> {
        CloudflareErrorResponse::try_from_response(&recorded_response(
            file_name,
            "https://example.com/wp-json/wp/v2/users/me",
        ))
    }

    /// Cloudflare's JSON body maps to the kind for its code and carries the ray
    /// ID and retry delay. 520–522, 525, 526, 1002, 1015 and 1016 are real
    /// failures recorded from a test site; the rest are Cloudflare's preview
    /// of the same response.
    #[rstest]
    #[case::internal(
        "cf-500-json.http",
        500,
        CloudflareErrorKind::CloudflareInternal,
        Some(30)
    )]
    #[case::bad_gateway("cf-502-json.http", 502, CloudflareErrorKind::OriginError, Some(60))]
    #[case::gateway_timeout("cf-504-json.http", 504, CloudflareErrorKind::OriginError, Some(120))]
    #[case::unknown_error("cf-520-json.http", 520, CloudflareErrorKind::OriginError, Some(60))]
    #[case::origin_down("cf-521-json.http", 521, CloudflareErrorKind::OriginDown, Some(120))]
    #[case::connect_timeout("cf-522-json.http", 522, CloudflareErrorKind::OriginTimeout, Some(120))]
    #[case::unreachable(
        "cf-523-json.http",
        523,
        CloudflareErrorKind::OriginUnreachable,
        Some(120)
    )]
    #[case::read_timeout("cf-524-json.http", 524, CloudflareErrorKind::OriginTimeout, Some(120))]
    #[case::ssl_handshake("cf-525-json.http", 525, CloudflareErrorKind::OriginSsl, None)]
    #[case::invalid_certificate("cf-526-json.http", 526, CloudflareErrorKind::OriginSsl, None)]
    #[case::rate_limited("cf-1015-json.http", 1015, CloudflareErrorKind::RateLimited, Some(10))]
    #[case::origin_dns(
        "cf-1016-json.http",
        1016,
        CloudflareErrorKind::OriginUnreachable,
        None
    )]
    #[case::disallowed_origin_ip(
        "cf-1002-json.http",
        1002,
        CloudflareErrorKind::OriginUnreachable,
        None
    )]
    #[case::firewall_rule("cf-1020-json.http", 1020, CloudflareErrorKind::AccessDenied, None)]
    fn recognises_cloudflare_json_errors(
        #[case] file_name: &str,
        #[case] code: u32,
        #[case] kind: CloudflareErrorKind,
        #[case] retry_after: Option<u64>,
    ) {
        let parsed = parse(file_name).expect("This is Cloudflare's own error");
        assert_eq!(parsed.cloudflare_code, Some(code));
        assert_eq!(parsed.kind, kind);
        assert_eq!(parsed.retry_after, retry_after);
        // The JSON's ray ID matches the header's, minus the data center.
        let ray_id = parsed.ray_id.expect("Every Cloudflare error has a ray ID");
        assert_eq!(ray_id.len(), 16, "{ray_id}");
    }

    #[rstest]
    #[case::origin_timeout("cf-522-plain-text.http", 522, CloudflareErrorKind::OriginTimeout)]
    #[case::firewall_rule("cf-1020-plain-text.http", 1020, CloudflareErrorKind::AccessDenied)]
    #[case::direct_ip_access("cf-1003-plain-text.http", 1003, CloudflareErrorKind::Other)]
    fn recognises_cloudflare_plain_text_errors(
        #[case] file_name: &str,
        #[case] code: u32,
        #[case] kind: CloudflareErrorKind,
    ) {
        let parsed = parse(file_name).expect("This is Cloudflare's own error");
        assert_eq!(parsed.cloudflare_code, Some(code));
        assert_eq!(parsed.kind, kind);
    }

    #[test]
    fn recognises_cloudflare_413_page_and_reads_ray_id_from_header() {
        assert_eq!(
            parse("cf-413-html.http"),
            Some(CloudflareErrorResponse {
                cloudflare_code: None,
                kind: CloudflareErrorKind::RequestTooLarge,
                ray_id: Some("a51300505b32e411".to_string()),
                retry_after: None,
            })
        );
    }

    /// The 502 page has no code, so it can't say the upload was too large even
    /// when that's why it was sent. Don't guess.
    #[test]
    fn recognises_cloudflare_502_page_as_an_origin_error() {
        let parsed = parse("cf-502-html.http").expect("This is Cloudflare's own error");
        assert_eq!(parsed.cloudflare_code, None);
        assert_eq!(parsed.kind, CloudflareErrorKind::OriginError);
    }

    /// What the login flow gets: it sends no `Accept` header, so Cloudflare
    /// answers with its branded HTML page rather than JSON.
    #[rstest]
    #[case::origin_timeout("cf-522-html.http", 522, CloudflareErrorKind::OriginTimeout)]
    #[case::origin_dns("cf-1016-html.http", 1016, CloudflareErrorKind::OriginUnreachable)]
    #[case::disallowed_origin_ip("cf-1002-html.http", 1002, CloudflareErrorKind::OriginUnreachable)]
    fn recognises_cloudflare_branded_error_pages(
        #[case] file_name: &str,
        #[case] code: u32,
        #[case] kind: CloudflareErrorKind,
    ) {
        let parsed = parse(file_name).expect("This is Cloudflare's own error");
        assert_eq!(parsed.cloudflare_code, Some(code));
        assert_eq!(parsed.kind, kind);
        assert!(parsed.ray_id.is_some());
    }

    #[rstest]
    #[case("<p>Error code 522</p>", Some(522))]
    #[case("properties:{errorCode: 1016 },", Some(1016))]
    #[case("properties:{errorCode:1002},", Some(1002))]
    #[case("Error code ", None)]
    #[case("Sorry, you have been blocked", None)]
    fn branded_error_page_code(#[case] body: &str, #[case] expected: Option<u32>) {
        assert_eq!(parse_branded_error_page_code(body), expected);
    }

    #[test]
    fn recognises_cloudflare_waf_block_page() {
        let parsed = parse("cf-waf-block-page.http").expect("This is Cloudflare's own error");
        assert_eq!(parsed.cloudflare_code, None);
        assert_eq!(parsed.kind, CloudflareErrorKind::AccessDenied);
    }

    #[test]
    fn recognises_cloudflare_challenge_page() {
        let parsed = parse("cf-challenge.http").expect("This is Cloudflare's own response");
        assert_eq!(parsed.kind, CloudflareErrorKind::Challenge);
        assert!(parsed.ray_id.is_some());
    }

    /// These came through Cloudflare, so they have its headers, but the site's
    /// own server wrote them.
    #[rstest]
    #[case::wordpress_error("origin-wp-error-behind-cloudflare.http")]
    #[case::nginx_403("origin-nginx-403-behind-cloudflare.http")]
    #[case::plain_text_forbidden("origin-plain-forbidden-behind-cloudflare.http")]
    #[case::apache_502("origin-apache-502-behind-cloudflare.http")]
    #[case::basic_auth_401("origin-basic-auth-401-behind-cloudflare.http")]
    #[case::not_behind_cloudflare("origin-nginx-413.http")]
    fn ignores_responses_the_site_wrote(#[case] file_name: &str) {
        assert_eq!(parse(file_name), None);
    }

    /// Cloudflare's body without Cloudflare's headers isn't trusted: any server
    /// can send `error code: 1020`.
    #[test]
    fn ignores_cloudflare_bodies_without_cloudflare_headers() {
        let mut response = recorded_response("cf-1020-plain-text.http", "https://example.com");
        response.response_header_map = Arc::new(WpNetworkHeaderMap::default());
        assert_eq!(CloudflareErrorResponse::try_from_response(&response), None);
    }

    #[test]
    fn ignores_successful_responses() {
        let mut response = recorded_response("cf-1020-plain-text.http", "https://example.com");
        response.status_code = 200;
        assert_eq!(CloudflareErrorResponse::try_from_response(&response), None);
    }

    #[rstest]
    #[case("error code: 1020", Some(1020))]
    #[case("error code: 1020\n", Some(1020))]
    #[case("error code: ", None)]
    #[case("error code: 10x", None)]
    #[case("Error code: 1020", None)]
    #[case("The error code: 1020", None)]
    fn plain_text_error_code(#[case] body: &str, #[case] expected: Option<u32>) {
        assert_eq!(parse_plain_text_error_code(body), expected);
    }

    #[rstest]
    #[case("a50c71565ed1595c-SEA", "a50c71565ed1595c")]
    #[case("a50c71565ed1595c", "a50c71565ed1595c")]
    #[case(" a50c71565ed1595c-YYC ", "a50c71565ed1595c")]
    fn ray_id_drops_the_data_center(#[case] header: &str, #[case] expected: &str) {
        assert_eq!(ray_id_without_data_center(header), expected);
    }
}
