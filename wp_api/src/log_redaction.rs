//! Redaction primitives for the diagnostic log line a client writes when a
//! request fails.
//!
//! A failed request is worth logging, but the two most useful fields — the
//! request URL and the response body — are also the two that can carry secrets
//! and personal data. These helpers reduce each field to a chosen level of
//! detail so a caller can log failures without deciding, at every call site,
//! what is safe to write down.

use serde_json::Value;
use url::Url;

/// How much of a request URL to write to a diagnostic log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpRequestUrlLogDetail {
    /// Scheme, host and path only. The query string is dropped entirely,
    /// including its keys.
    PathOnly,
    /// Query keys are kept, their values replaced with `REDACTED`. Which
    /// parameters a request carried is usually enough to identify it, and the
    /// values are where the secrets and the personal data are.
    ///
    /// `rest_route` keeps its value: on a site using plain permalinks that
    /// parameter carries the REST route itself, so redacting it would leave no
    /// indication of which endpoint failed.
    QueryKeysOnly,
    /// The URL as it was sent, except for the query parameters that are
    /// redacted in every mode (see [`redact_request_url_for_log`]).
    Full,
}

/// How much of a failed response's body to write to a diagnostic log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpResponseBodyLogDetail {
    /// The body is left out of the log line altogether.
    Omitted,
    /// A description of the body's size and shape, without its contents.
    Summary,
    /// The body as it was received.
    Full,
}

/// Query parameters whose values are replaced with `REDACTED` at every
/// [`WpRequestUrlLogDetail`], including [`WpRequestUrlLogDetail::Full`].
///
/// `token` and `password` are carried in URLs this crate builds: `GET
/// /oauth2/token-info` sends the WordPress.com access token as `token`, and
/// `GET /wp/v2/posts/<id>` sends a password-protected post's password as
/// `password`. The remaining names are the usual spellings of the same kinds
/// of secret, listed so that a URL assembled elsewhere is covered too.
const ALWAYS_REDACTED_QUERY_KEYS: &[&str] = &[
    "_wpnonce",
    "access_token",
    "client_secret",
    "code",
    "password",
    "pw",
    "secret",
    "token",
];

/// Query parameters that identify the endpoint rather than describe the
/// request, so their values survive [`WpRequestUrlLogDetail::QueryKeysOnly`].
const ENDPOINT_QUERY_KEYS: &[&str] = &["rest_route"];

const REDACTED_VALUE: &str = "REDACTED";
const UNPARSABLE_URL: &str = "<unparsable URL>";

/// The most keys [`WpResponseBodyLogDetail::Summary`] lists before it counts
/// the rest, so that a summary stays a single readable line.
const MAX_SUMMARIZED_KEYS: usize = 20;

/// Reduces `url` to the requested level of detail for a diagnostic log line.
///
/// Three things are removed at every level, because they are secrets wherever
/// they appear: credentials in the authority (`https://user:pass@host/`), the
/// fragment (an OAuth2 implicit-flow redirect returns the access token in it),
/// and the values of [`ALWAYS_REDACTED_QUERY_KEYS`].
///
/// A URL that cannot be parsed cannot be redacted, so none of it is returned.
#[uniffi::export]
pub fn redact_request_url_for_log(url: &str, detail: WpRequestUrlLogDetail) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return UNPARSABLE_URL.to_string();
    };

    // Both setters fail only on URLs that cannot have an authority (`mailto:`,
    // `data:`), which carry no credentials to remove in the first place.
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_fragment(None);

    match detail {
        WpRequestUrlLogDetail::PathOnly => parsed.set_query(None),
        WpRequestUrlLogDetail::QueryKeysOnly => {
            redact_query_values(&mut parsed, |key| !is_endpoint_key(key))
        }
        WpRequestUrlLogDetail::Full => redact_query_values(&mut parsed, is_always_redacted_key),
    }
    parsed.to_string()
}

/// Replaces the value of every query parameter `should_redact` selects, leaving
/// the keys and their order as they were.
fn redact_query_values(url: &mut Url, should_redact: impl Fn(&str) -> bool) {
    if url.query().is_none() {
        return;
    }

    let redacted: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| {
            let value = if should_redact(&key) {
                REDACTED_VALUE.to_string()
            } else {
                value.into_owned()
            };
            (key.into_owned(), value)
        })
        .collect();

    url.query_pairs_mut().clear().extend_pairs(redacted);
    // `clear()` leaves a bare `?` behind when the query held no pairs.
    if url.query() == Some("") {
        url.set_query(None);
    }
}

/// Reduces a failed response's `body` to the requested level of detail, or
/// `None` when the body does not belong in the log line at all.
#[uniffi::export]
pub fn summarize_response_body_for_log(
    body: &str,
    detail: WpResponseBodyLogDetail,
) -> Option<String> {
    match detail {
        WpResponseBodyLogDetail::Omitted => None,
        WpResponseBodyLogDetail::Summary => Some(summarize_body(body)),
        WpResponseBodyLogDetail::Full => Some(body.to_string()),
    }
}

fn is_always_redacted_key(key: &str) -> bool {
    ALWAYS_REDACTED_QUERY_KEYS
        .iter()
        .any(|candidate| key.eq_ignore_ascii_case(candidate))
}

fn is_endpoint_key(key: &str) -> bool {
    ENDPOINT_QUERY_KEYS
        .iter()
        .any(|candidate| key.eq_ignore_ascii_case(candidate))
}

/// Describes a body's size and shape without repeating anything it contains.
///
/// Object keys are named because they are schema rather than data, and they
/// are what tells a `{"code","message","data"}` error apart from a truncated
/// payload. Everything else reduces to a size and a shape — an HTML error page
/// or a partial upload has no keys to report.
fn summarize_body(body: &str) -> String {
    if body.is_empty() {
        return "<empty>".to_string();
    }
    let byte_count = body.len();
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(map)) if map.is_empty() => {
            format!("<{byte_count} bytes, empty JSON object>")
        }
        Ok(Value::Object(map)) => {
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable();
            let listed = keys
                .iter()
                .take(MAX_SUMMARIZED_KEYS)
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            match keys.len().checked_sub(MAX_SUMMARIZED_KEYS) {
                Some(remaining) if remaining > 0 => format!(
                    "<{byte_count} bytes, JSON object with keys: {listed} and {remaining} more>"
                ),
                _ => format!("<{byte_count} bytes, JSON object with keys: {listed}>"),
            }
        }
        Ok(Value::Array(items)) => {
            format!("<{byte_count} bytes, JSON array of {} items>", items.len())
        }
        Ok(_) => format!("<{byte_count} bytes, JSON value>"),
        Err(_) => format!("<{byte_count} bytes, not JSON>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::keeps_scheme_host_and_path(
        "https://example.com/wp-json/wp/v2/posts?per_page=10",
        "https://example.com/wp-json/wp/v2/posts"
    )]
    #[case::drops_keys_as_well_as_values(
        "https://example.com/wp-json/wp/v2/posts?search=my+secret+draft",
        "https://example.com/wp-json/wp/v2/posts"
    )]
    #[case::drops_the_rest_route_endpoint_too(
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fposts",
        "https://example.com/index.php"
    )]
    fn path_only_drops_the_whole_query(#[case] url: &str, #[case] expected: &str) {
        assert_eq!(
            redact_request_url_for_log(url, WpRequestUrlLogDetail::PathOnly),
            expected
        );
    }

    #[rstest]
    #[case::keeps_keys_drops_values(
        "https://example.com/wp-json/wp/v2/posts?per_page=10&search=vacation",
        "https://example.com/wp-json/wp/v2/posts?per_page=REDACTED&search=REDACTED"
    )]
    #[case::keeps_the_rest_route_value(
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fposts&per_page=10",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fposts&per_page=REDACTED"
    )]
    fn query_keys_only_keeps_keys(#[case] url: &str, #[case] expected: &str) {
        assert_eq!(
            redact_request_url_for_log(url, WpRequestUrlLogDetail::QueryKeysOnly),
            expected
        );
    }

    #[rstest]
    #[case::ordinary_parameters_survive(
        "https://example.com/wp-json/wp/v2/posts?per_page=10&search=vacation",
        "https://example.com/wp-json/wp/v2/posts?per_page=10&search=vacation"
    )]
    // `GET /oauth2/token-info` sends the WordPress.com access token as `token`.
    #[case::token_info_access_token(
        "https://public-api.wordpress.com/oauth2/token-info?client_id=11&token=abc123",
        "https://public-api.wordpress.com/oauth2/token-info?client_id=11&token=REDACTED"
    )]
    // `GET /wp/v2/posts/<id>` sends a protected post's password as `password`.
    #[case::password_protected_post(
        "https://example.com/wp-json/wp/v2/posts/7?password=hunter2",
        "https://example.com/wp-json/wp/v2/posts/7?password=REDACTED"
    )]
    #[case::matches_keys_case_insensitively(
        "https://example.com/callback?Access_Token=abc123",
        "https://example.com/callback?Access_Token=REDACTED"
    )]
    fn full_redacts_only_the_always_redacted_keys(#[case] url: &str, #[case] expected: &str) {
        assert_eq!(
            redact_request_url_for_log(url, WpRequestUrlLogDetail::Full),
            expected
        );
    }

    #[rstest]
    #[case(WpRequestUrlLogDetail::PathOnly)]
    #[case(WpRequestUrlLogDetail::QueryKeysOnly)]
    #[case(WpRequestUrlLogDetail::Full)]
    fn credentials_in_the_authority_are_removed_at_every_detail(
        #[case] detail: WpRequestUrlLogDetail,
    ) {
        let redacted =
            redact_request_url_for_log("https://admin:hunter2@example.com/wp-json", detail);
        assert_eq!(redacted, "https://example.com/wp-json");
    }

    #[rstest]
    #[case(WpRequestUrlLogDetail::PathOnly)]
    #[case(WpRequestUrlLogDetail::QueryKeysOnly)]
    #[case(WpRequestUrlLogDetail::Full)]
    fn the_fragment_is_removed_at_every_detail(#[case] detail: WpRequestUrlLogDetail) {
        // An OAuth2 implicit-flow redirect returns the access token in the fragment.
        let redacted = redact_request_url_for_log(
            "https://example.com/callback#access_token=abc123&token_type=bearer",
            detail,
        );
        assert_eq!(redacted, "https://example.com/callback");
    }

    #[rstest]
    #[case(WpRequestUrlLogDetail::PathOnly)]
    #[case(WpRequestUrlLogDetail::QueryKeysOnly)]
    #[case(WpRequestUrlLogDetail::Full)]
    fn an_unparsable_url_is_not_logged(#[case] detail: WpRequestUrlLogDetail) {
        assert_eq!(
            redact_request_url_for_log("not a url", detail),
            "<unparsable URL>"
        );
    }

    #[rstest]
    #[case(WpRequestUrlLogDetail::QueryKeysOnly)]
    #[case(WpRequestUrlLogDetail::Full)]
    fn a_url_without_a_query_gains_no_question_mark(#[case] detail: WpRequestUrlLogDetail) {
        assert_eq!(
            redact_request_url_for_log("https://example.com/wp-json/wp/v2/posts", detail),
            "https://example.com/wp-json/wp/v2/posts"
        );
    }

    #[test]
    fn omitted_leaves_the_body_out() {
        assert_eq!(
            summarize_response_body_for_log(r#"{"code":"x"}"#, WpResponseBodyLogDetail::Omitted),
            None
        );
    }

    #[test]
    fn full_returns_the_body_unchanged() {
        let body = r#"{"code":"rest_forbidden","message":"Sorry"}"#;
        assert_eq!(
            summarize_response_body_for_log(body, WpResponseBodyLogDetail::Full),
            Some(body.to_string())
        );
    }

    #[rstest]
    #[case::wordpress_error_body(
        r#"{"code":"rest_forbidden","message":"Sorry, you are not allowed to do that.","data":{"status":401}}"#,
        "<98 bytes, JSON object with keys: code, data, message>"
    )]
    #[case::empty_object("{}", "<2 bytes, empty JSON object>")]
    #[case::array("[1,2,3]", "<7 bytes, JSON array of 3 items>")]
    #[case::scalar(r#""just a string""#, "<15 bytes, JSON value>")]
    #[case::html_error_page(
        "<html><body>Fatal error: out of memory</body></html>",
        "<52 bytes, not JSON>"
    )]
    #[case::empty_body("", "<empty>")]
    fn summary_describes_shape_without_contents(#[case] body: &str, #[case] expected: &str) {
        assert_eq!(
            summarize_response_body_for_log(body, WpResponseBodyLogDetail::Summary),
            Some(expected.to_string())
        );
    }

    #[test]
    fn summary_counts_keys_beyond_the_listed_maximum() {
        let fields: Vec<String> = (0..MAX_SUMMARIZED_KEYS + 3)
            .map(|i| format!(r#""key{i:02}":{i}"#))
            .collect();
        let body = format!("{{{}}}", fields.join(","));

        let summary = summarize_response_body_for_log(&body, WpResponseBodyLogDetail::Summary)
            .expect("Summary should always produce a description");
        assert!(
            summary.ends_with("key19 and 3 more>"),
            "expected the summary to list 20 keys and count the rest, got: {summary}"
        );
    }

    #[test]
    fn summary_does_not_repeat_object_values() {
        let summary = summarize_response_body_for_log(
            r#"{"email":"person@example.com","token":"abc123"}"#,
            WpResponseBodyLogDetail::Summary,
        )
        .expect("Summary should always produce a description");
        assert!(!summary.contains("person@example.com"), "got: {summary}");
        assert!(!summary.contains("abc123"), "got: {summary}");
    }
}
