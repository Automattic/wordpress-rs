//! The diagnostic log line a client writes when a request fails.
//!
//! A failed request is worth logging, but the two most useful fields — the
//! request URL and the response body — are also the two that can carry secrets
//! and personal data. A [`WpRequestErrorLogPolicy`] reduces each to a chosen
//! level of detail, so a caller decides once what is safe to write down rather
//! than at every call site.
//!
//! The line itself is composed here rather than in each set of bindings, so
//! that every platform reports a failure the same way.

use crate::api_error::{RequestExecutionError, WpApiError, WpErrorCode};
use crate::login::url_discovery::{
    AutoDiscoveryAttemptFailure, FetchAndParseApiRootFailure, FindApiRootFailure,
};
use serde_json::Value;
use url::Url;

/// How much of a failed request is written to a diagnostic log line.
///
/// [`request_url`](Self::request_url) governs the `url=` field.
/// [`response_body`](Self::response_body) governs every field drawn from the
/// response: the body itself, the `message` a `WpError` carries, and the
/// `reason` a response failed to parse with.
///
/// Neither reaches the local file path on a media failure, nor the platform
/// error text inside a request execution failure — those come from neither the
/// URL nor the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct WpRequestErrorLogPolicy {
    pub request_url: WpRequestUrlLogDetail,
    pub response_body: WpResponseBodyLogDetail,
}

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
    ///
    /// No credential survives this, but personal data does: a comment list
    /// sends `author_email`, and every list endpoint sends whatever the user
    /// typed as `search`. Choose [`QueryKeysOnly`](Self::QueryKeysOnly) for a
    /// client that should record neither.
    Full,
}

/// How much of a failed response to write to a diagnostic log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpResponseBodyLogDetail {
    /// Nothing the response carried appears in the line, beyond the error code
    /// it was parsed into. The code names the failure rather than describing
    /// it, and without it the line says only that a request failed.
    Omitted,
    /// The response's own account of the failure — the `message` a `WpError`
    /// carries, or the reason a body could not be parsed — plus the size and
    /// shape of the body itself.
    ///
    /// A server writes those messages, so one can name a user: WordPress
    /// echoes an invalid parameter's value, and a plugin may filter `message`
    /// into anything. What it will not carry is a credential the request sent.
    /// At least it's not supposed to.
    Summary,
    /// The above, and the body verbatim.
    ///
    /// A `WpError` reports no separate body: its `code` and `message` are the
    /// body's own fields, already named above.
    Full,
}

/// Substrings that make a query parameter's name a secret whatever it is
/// prefixed or suffixed with, so `refresh_token` and `id_token` are covered
/// alongside `access_token`. A key containing one of these is not expected to be innocent.
const ALWAYS_REDACTED_KEY_SUBSTRINGS: &[&str] = &["token", "secret", "password"];

/// Query parameter names that are secrets in full but whose substrings are
/// not — `code` also appears in `country_code` and `postal_code`, which are
/// worth keeping legible.
///
/// `token` and `password` are carried in URLs this crate builds: `GET
/// /oauth2/token-info` sends the WordPress.com access token as `token`, and
/// `GET /wp/v2/posts/<id>` and `GET /wp/v2/comments` send a password-protected
/// post's password as `password`. Both are matched by
/// [`ALWAYS_REDACTED_KEY_SUBSTRINGS`]. The names below are the usual spellings
/// of the same kinds of secret, listed so that a URL assembled elsewhere is
/// covered too.
const ALWAYS_REDACTED_QUERY_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "auth",
    "authorization",
    "code",
    "hmac",
    "nonce",
    "passwd",
    "pw",
    "pwd",
    "session",
    "sig",
    "signature",
    "_wpnonce",
];

/// Query parameters that identify the endpoint rather than describe the
/// request, so their values survive [`WpRequestUrlLogDetail::QueryKeysOnly`].
const ENDPOINT_QUERY_KEYS: &[&str] = &["rest_route"];

const REDACTED_VALUE: &str = "REDACTED";
const UNPARSABLE_URL: &str = "<unparsable URL>";

/// The most keys [`WpResponseBodyLogDetail::Summary`] lists before it counts
/// the rest, so that a summary stays a single readable line.
const MAX_SUMMARIZED_KEYS: usize = 20;

/// The most server-supplied text any one field contributes to a log line.
/// Generous enough for a stack trace or an HTML error page's opening, short
/// enough that a hostile site cannot flood a crash report.
const MAX_LOGGED_TEXT_BYTES: usize = 8192;

/// The bound for server-supplied text that names something rather than
/// explaining it — a JSON object's key, or an error code outside
/// [`WpErrorCode`]'s own variants. Anything longer is not a name.
const MAX_LOGGED_IDENTIFIER_BYTES: usize = 64;

/// Reduces `url` to the requested level of detail for a diagnostic log line.
///
/// Three things are removed at every level, because they are secrets wherever
/// they appear: credentials in the authority (`https://user:pass@host/`), the
/// fragment (an OAuth2 implicit-flow redirect returns the access token in it),
/// and the value of any query parameter named by
/// `ALWAYS_REDACTED_KEY_SUBSTRINGS` or `ALWAYS_REDACTED_QUERY_KEYS`, which is
/// where those names and the reasoning behind them live. Matching ignores case
/// and a `[]` suffix.
///
/// A URL that cannot be parsed cannot be redacted, so none of it is returned.
/// An IDN host comes back punycoded, which is what the request used.
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
///
/// Rewriting a query re-encodes it (`%20` becomes `+`), so a query with nothing
/// to redact is left exactly as it arrived rather than round-tripped.
fn redact_query_values(url: &mut Url, should_redact: impl Fn(&str) -> bool) {
    if !url.query_pairs().any(|(key, _)| should_redact(&key)) {
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
        WpResponseBodyLogDetail::Full => Some(fit_to_one_log_line(body)),
    }
}

/// Passes through free text taken out of a failed response — the `message` a
/// `WpError` carries, or the reason a response failed to parse — unless the
/// response is being left out of the line altogether.
///
/// Such text has no shape to summarize, so it has no middle setting: it is the
/// response's account of what went wrong, and it is either reported or it is
/// not. It is also the one thing in the line a server chooses freely, so what
/// passes through is fitted to a single line.
#[uniffi::export]
pub fn redact_response_text_for_log(text: &str, detail: WpResponseBodyLogDetail) -> Option<String> {
    match detail {
        WpResponseBodyLogDetail::Omitted => None,
        WpResponseBodyLogDetail::Summary | WpResponseBodyLogDetail::Full => {
            Some(fit_to_one_log_line(text))
        }
    }
}

/// Keeps server-supplied text to one bounded log line: line breaks are escaped
/// so it cannot forge a second entry, and anything past
/// [`MAX_LOGGED_TEXT_BYTES`] is dropped and counted.
fn fit_to_one_log_line(text: &str) -> String {
    fit_to_log_line(text, MAX_LOGGED_TEXT_BYTES)
}

/// As [`fit_to_one_log_line`], to a caller-chosen bound.
///
/// Truncation lands on a character boundary, so the result is never a partial
/// UTF-8 sequence.
fn fit_to_log_line(text: &str, max_bytes: usize) -> String {
    let escaped = text.replace('\r', "\\r").replace('\n', "\\n");
    if escaped.len() <= max_bytes {
        return escaped;
    }

    let mut end = max_bytes;
    while end > 0 && !escaped.is_char_boundary(end) {
        end -= 1;
    }
    let dropped = escaped.len() - end;
    format!("{} …({dropped} more bytes)", &escaped[..end])
}

/// Lowercases a query key and drops a `[]` suffix, so that the PHP array form
/// (`tlds[]`, which this crate builds for domain searches) is matched by the
/// same name as its scalar spelling.
fn normalize_query_key(key: &str) -> &str {
    key.strip_suffix("[]").unwrap_or(key)
}

fn is_always_redacted_key(key: &str) -> bool {
    let key = normalize_query_key(key).to_ascii_lowercase();
    ALWAYS_REDACTED_KEY_SUBSTRINGS
        .iter()
        .any(|candidate| key.contains(candidate))
        || ALWAYS_REDACTED_QUERY_KEYS.contains(&key.as_str())
}

fn is_endpoint_key(key: &str) -> bool {
    let key = normalize_query_key(key);
    ENDPOINT_QUERY_KEYS
        .iter()
        .any(|candidate| key.eq_ignore_ascii_case(candidate))
}

/// Describes a body's size and shape without repeating anything it contains.
///
/// Object keys are named because they are usually schema rather than data, and
/// they are what tells a `{"code","message","data"}` error apart from a
/// truncated payload. Everything else reduces to a size and a shape — an HTML
/// error page or a partial upload has no keys to report.
///
/// A key is still chosen by whoever wrote the response, so each is fitted to
/// the line like any other server text: a body keyed by a megabyte of text, or
/// by a newline, can neither flood a crash report nor forge a second entry.
///
/// The byte count is of the body as decoded, which for a response that was not
/// valid UTF-8 differs from what arrived on the wire.
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
                .map(|key| fit_to_log_line(key, MAX_LOGGED_IDENTIFIER_BYTES))
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

/// A concise description of a failed request, for logs and crash reporting
/// ONLY. Never surface it to users; localize the error instead.
#[uniffi::export]
pub fn wp_api_error_log_description(error: &WpApiError, policy: WpRequestErrorLogPolicy) -> String {
    match error {
        WpApiError::WpError {
            error_code,
            error_message,
            status_code,
            request_url,
            request_method,
            ..
        } => format!(
            "WpError(code={}, status={status_code}{}, method={request_method:?}, url={})",
            error_code_description(error_code),
            text_field("message", error_message, policy),
            url_field(request_url, policy),
        ),
        WpApiError::InvalidHttpStatusCode {
            status_code,
            request_url,
            request_method,
        } => format!(
            "InvalidHttpStatusCode(status={status_code}, method={request_method:?}, url={})",
            url_field(request_url, policy),
        ),
        WpApiError::RequestExecutionFailed {
            status_code,
            reason,
            request_url,
            request_method,
            ..
        } => format!(
            "RequestExecutionFailed(status={}, reason={reason:?}, method={request_method:?}, url={})",
            optional_status(*status_code),
            url_field(request_url, policy),
        ),
        WpApiError::MediaFileNotFound { file_path } => {
            format!("MediaFileNotFound(path={file_path})")
        }
        WpApiError::MediaFileUnreadable { file_path } => {
            format!("MediaFileUnreadable(path={file_path})")
        }
        WpApiError::SiteUrlParsingError { reason } => {
            format!("SiteUrlParsingError(reason={reason})")
        }
        WpApiError::ResponseParsingError {
            reason,
            response,
            request_url,
            request_method,
        } => format!(
            "ResponseParsingError(method={request_method:?}{}, url={}{})",
            text_field("reason", reason, policy),
            url_field(request_url, policy),
            response_field(response, policy),
        ),
        WpApiError::UnknownError {
            status_code,
            response,
            request_url,
            request_method,
        } => format!(
            "UnknownError(status={status_code}, method={request_method:?}, url={}{})",
            url_field(request_url, policy),
            response_field(response, policy),
        ),
    }
}

/// A concise description of a failed API discovery attempt, for logs and crash
/// reporting ONLY.
#[uniffi::export]
pub fn auto_discovery_failure_log_description(
    failure: &AutoDiscoveryAttemptFailure,
    policy: WpRequestErrorLogPolicy,
) -> String {
    match failure {
        // The site URL is what failed to parse, so there is no URL to redact
        // and report; the parse error names no part of the input.
        AutoDiscoveryAttemptFailure::ParseSiteUrl { error } => {
            format!("ParseSiteUrl(reason={error:?})")
        }
        AutoDiscoveryAttemptFailure::FindApiRoot {
            parsed_site_url,
            find_api_root_failure,
        } => format!(
            "FindApiRoot(siteUrl={}, reason={})",
            url_field(parsed_site_url.as_str(), policy),
            find_api_root_failure_description(find_api_root_failure, policy),
        ),
        AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
            parsed_site_url,
            api_root_url,
            fetch_and_parse_api_root_failure,
        } => format!(
            "FetchAndParseApiRoot(siteUrl={}, apiRootUrl={}, reason={})",
            url_field(parsed_site_url.as_str(), policy),
            url_field(api_root_url.as_str(), policy),
            fetch_and_parse_description(fetch_and_parse_api_root_failure, policy),
        ),
    }
}

fn find_api_root_failure_description(
    failure: &FindApiRootFailure,
    policy: WpRequestErrorLogPolicy,
) -> String {
    match failure {
        FindApiRootFailure::FetchHomepage { error } => format!(
            "FetchHomepage({})",
            request_execution_error_description(error, policy)
        ),
        FindApiRootFailure::ProbablyNotAWordPressSite => "ProbablyNotAWordPressSite".to_string(),
        FindApiRootFailure::RestApiDisabled => "RestApiDisabled".to_string(),
    }
}

fn fetch_and_parse_description(
    failure: &FetchAndParseApiRootFailure,
    policy: WpRequestErrorLogPolicy,
) -> String {
    match failure {
        FetchAndParseApiRootFailure::FetchApiRoot { error } => format!(
            "FetchApiRoot({})",
            request_execution_error_description(error, policy)
        ),
        FetchAndParseApiRootFailure::ParseApiRoot {
            parsing_error_message,
            response_body,
            response_body_type,
            reason,
        } => format!(
            "ParseApiRoot(bodyType={response_body_type:?}, reason={reason:?}{}{})",
            text_field("parsingError", parsing_error_message, policy),
            response_field(response_body, policy),
        ),
        FetchAndParseApiRootFailure::WpError {
            error_code,
            error_message,
            status_code,
        } => format!(
            "WpError(code={}, status={status_code}{})",
            error_code_description(error_code),
            text_field("message", error_message, policy),
        ),
        // `api_details` is the whole parsed API root; only the reason is worth
        // a log line.
        FetchAndParseApiRootFailure::ApplicationPasswordsNotSupported { reason, .. } => {
            format!("ApplicationPasswordsNotSupported(reason={reason:?})")
        }
    }
}

/// Not exported: a request execution failure reaches the bindings inside a
/// `WpApiError` or an `AutoDiscoveryAttemptFailure`, never on its own.
fn request_execution_error_description(
    error: &RequestExecutionError,
    policy: WpRequestErrorLogPolicy,
) -> String {
    match error {
        RequestExecutionError::RequestExecutionFailed {
            status_code,
            reason,
            request_url,
            request_method,
            ..
        } => format!(
            "RequestExecutionFailed(status={}, reason={reason:?}, method={request_method:?}, url={})",
            optional_status(*status_code),
            url_field(request_url, policy),
        ),
        RequestExecutionError::MediaFileNotFound { file_path } => {
            format!("MediaFileNotFound(path={file_path})")
        }
        RequestExecutionError::MediaFileUnreadable { file_path } => {
            format!("MediaFileUnreadable(path={file_path})")
        }
    }
}

fn url_field(request_url: &str, policy: WpRequestErrorLogPolicy) -> String {
    redact_request_url_for_log(request_url, policy.request_url)
}

/// Names the error code, at every policy: it is the field that says what
/// failed, and unlike the response's `message` it is an identifier rather than
/// prose.
///
/// Every variant but one is a fixed name. [`WpErrorCode::CustomError`] holds
/// the response's `code` field verbatim, for any code outside the enum, so it
/// is fitted to the line — a plugin is free to put a newline in it, which would
/// otherwise split the entry in two.
fn error_code_description(error_code: &WpErrorCode) -> String {
    match error_code {
        WpErrorCode::CustomError(code) => format!(
            "CustomError({})",
            fit_to_log_line(code, MAX_LOGGED_IDENTIFIER_BYTES)
        ),
        named => format!("{named:?}"),
    }
}

/// The `, response=…` portion of a line, or nothing when the policy leaves the
/// body out.
fn response_field(response: &str, policy: WpRequestErrorLogPolicy) -> String {
    summarize_response_body_for_log(response, policy.response_body)
        .map(|summary| format!(", response={summary}"))
        .unwrap_or_default()
}

/// A `, name=…` portion carrying free text the response supplied, or nothing
/// when the policy is not logging the body.
fn text_field(name: &str, text: &str, policy: WpRequestErrorLogPolicy) -> String {
    redact_response_text_for_log(text, policy.response_body)
        .map(|text| format!(", {name}={text}"))
        .unwrap_or_default()
}

/// A status code that a transport failure may not have reached the server to
/// receive.
fn optional_status(status_code: Option<u32>) -> String {
    status_code.map_or_else(|| "none".to_string(), |code| code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_error::{RequestExecutionErrorReason, WpErrorCode};
    use crate::parsed_url::{ParseUrlError, ParsedUrl};
    use crate::request::endpoint::WpEndpointUrl;
    use crate::request::{RequestMethod, ResponseBodyType, WpNetworkHeaderMap, WpNetworkResponse};
    use rstest::rstest;
    use std::sync::Arc;

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
    // `query_pairs()` decodes before the name is matched, so an encoded key
    // cannot smuggle a secret past the list.
    #[case::matches_percent_encoded_keys(
        "https://example.com/callback?%74oken=abc123",
        "https://example.com/callback?token=REDACTED"
    )]
    // The PHP array form this crate builds for domain searches (`tlds[]`).
    #[case::matches_the_php_array_form(
        "https://example.com/callback?token[]=abc123",
        "https://example.com/callback?token%5B%5D=REDACTED"
    )]
    #[case::matches_any_name_containing_a_secret_word(
        "https://example.com/callback?refresh_token=abc&id_token=def",
        "https://example.com/callback?refresh_token=REDACTED&id_token=REDACTED"
    )]
    // `code` is a secret in full but not as a substring, so a country code
    // stays legible.
    #[case::does_not_over_match_code(
        "https://example.com/checkout?country_code=US&code=abc123",
        "https://example.com/checkout?country_code=US&code=REDACTED"
    )]
    #[case::redacts_every_occurrence_of_a_repeated_key(
        "https://example.com/callback?token=one&token=two",
        "https://example.com/callback?token=REDACTED&token=REDACTED"
    )]
    #[case::keeps_the_port(
        "https://example.com:8443/wp-json/wp/v2/posts/7?password=hunter2",
        "https://example.com:8443/wp-json/wp/v2/posts/7?password=REDACTED"
    )]
    fn full_redacts_only_the_always_redacted_keys(#[case] url: &str, #[case] expected: &str) {
        assert_eq!(
            redact_request_url_for_log(url, WpRequestUrlLogDetail::Full),
            expected
        );
    }

    #[rstest]
    #[case(WpRequestUrlLogDetail::PathOnly, "https://example.com/wp-json")]
    #[case(
        WpRequestUrlLogDetail::QueryKeysOnly,
        "https://example.com/wp-json?search=REDACTED"
    )]
    #[case(WpRequestUrlLogDetail::Full, "https://example.com/wp-json?search=cats")]
    fn credentials_in_the_authority_are_removed_at_every_detail(
        #[case] detail: WpRequestUrlLogDetail,
        #[case] expected: &str,
    ) {
        let redacted = redact_request_url_for_log(
            "https://admin:hunter2@example.com/wp-json?search=cats",
            detail,
        );
        assert_eq!(redacted, expected);
    }

    #[test]
    fn full_leaves_a_query_with_nothing_to_redact_exactly_as_it_arrived() {
        // Rewriting the query would re-encode it — `%20` would come back as
        // `+` — so a query holding no secret is passed through untouched.
        let url = "https://example.com/wp-json/wp/v2/posts?search=two%20words";
        assert_eq!(
            redact_request_url_for_log(url, WpRequestUrlLogDetail::Full),
            url
        );
    }

    #[rstest]
    #[case::bare_question_mark("https://example.com/wp-json?", "https://example.com/wp-json?")]
    #[case::key_without_a_value(
        "https://example.com/wp-json?draft",
        "https://example.com/wp-json?draft=REDACTED"
    )]
    fn query_keys_only_handles_a_query_that_is_not_key_value_pairs(
        #[case] url: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(
            redact_request_url_for_log(url, WpRequestUrlLogDetail::QueryKeysOnly),
            expected
        );
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
    fn free_text_from_a_response_is_kept_back_only_when_the_response_is_omitted() {
        assert_eq!(
            redact_response_text_for_log(
                "person@example.com is not authorized",
                WpResponseBodyLogDetail::Omitted
            ),
            None
        );
    }

    #[rstest]
    #[case(WpResponseBodyLogDetail::Summary)]
    #[case(WpResponseBodyLogDetail::Full)]
    fn free_text_survives_when_the_response_is_reported_at_all(
        #[case] detail: WpResponseBodyLogDetail,
    ) {
        assert_eq!(
            redact_response_text_for_log("Sorry, you are not allowed", detail),
            Some("Sorry, you are not allowed".to_string())
        );
    }

    #[test]
    fn a_full_body_cannot_forge_a_second_log_line() {
        // A site controls its response body, so a newline in it would otherwise
        // read as a separate log entry.
        let logged = summarize_response_body_for_log(
            "first\nE/Forged: second",
            WpResponseBodyLogDetail::Full,
        )
        .expect("Full should always produce a value");
        assert!(!logged.contains('\n'), "got: {logged}");
        assert!(logged.contains("first\\nE/Forged: second"), "got: {logged}");
    }

    #[test]
    fn a_summarized_key_cannot_forge_a_second_log_line() {
        // A key is as much the site's choice as the body around it, and
        // `Summary` is the default policy rather than an opt-in.
        let logged = summarize_response_body_for_log(
            "{\"a\\nE/Forged: second\":1}",
            WpResponseBodyLogDetail::Summary,
        )
        .expect("Summary should always produce a value");
        assert!(!logged.contains('\n'), "got: {logged}");
    }

    #[test]
    fn a_summarized_key_is_capped() {
        // `MAX_SUMMARIZED_KEYS` bounds how many keys are listed; without a
        // bound on each, one key is enough to flood a crash report.
        let body = format!(r#"{{"{}":1}}"#, "k".repeat(100_000));
        let logged = summarize_response_body_for_log(&body, WpResponseBodyLogDetail::Summary)
            .expect("Summary should always produce a value");
        assert!(
            logged.len() < 200,
            "one key should not carry the line away, got {} bytes",
            logged.len()
        );
    }

    #[test]
    fn a_full_body_is_capped_and_the_dropped_bytes_counted() {
        let body = "x".repeat(MAX_LOGGED_TEXT_BYTES + 50);
        let logged = summarize_response_body_for_log(&body, WpResponseBodyLogDetail::Full)
            .expect("Full should always produce a value");
        assert!(
            logged.ends_with("…(50 more bytes)"),
            "got the tail: {}",
            &logged[logged.len() - 40..]
        );
    }

    #[test]
    fn capping_a_full_body_does_not_split_a_character() {
        // The leading ASCII byte puts the cap in the middle of a two-byte
        // character, which is the case the boundary walk exists for. Without
        // it, 8192 lands on a boundary anyway and the walk never runs.
        let body = format!("x{}", "é".repeat(MAX_LOGGED_TEXT_BYTES));
        let logged = summarize_response_body_for_log(&body, WpResponseBodyLogDetail::Full)
            .expect("Full should always produce a value");
        assert!(logged.ends_with("more bytes)"), "got: {logged}");
        assert!(
            logged.len() < body.len(),
            "the body should have been capped"
        );
    }

    const ACCESS_TOKEN: &str = "s3cr3t-access-token";
    const PERSONAL_DATA: &str = "person@example.com";
    const TOKEN_INFO_URL: &str =
        "https://public-api.wordpress.com/oauth2/token-info?client_id=11&token=s3cr3t-access-token";
    const ERROR_BODY: &str =
        r#"{"code":"invalid_token","message":"person@example.com is not authorized"}"#;

    /// Mirrors the default the bindings apply: a failure is described as fully
    /// as it can be without recording a credential.
    const DEFAULT_POLICY: WpRequestErrorLogPolicy = WpRequestErrorLogPolicy {
        request_url: WpRequestUrlLogDetail::Full,
        response_body: WpResponseBodyLogDetail::Summary,
    };
    /// The privacy-oriented alternative, for a client that should record no
    /// value at all.
    const PRIVATE_POLICY: WpRequestErrorLogPolicy = WpRequestErrorLogPolicy {
        request_url: WpRequestUrlLogDetail::QueryKeysOnly,
        response_body: WpResponseBodyLogDetail::Omitted,
    };
    const STRICT_POLICY: WpRequestErrorLogPolicy = WpRequestErrorLogPolicy {
        request_url: WpRequestUrlLogDetail::PathOnly,
        response_body: WpResponseBodyLogDetail::Omitted,
    };
    const FULL_POLICY: WpRequestErrorLogPolicy = WpRequestErrorLogPolicy {
        request_url: WpRequestUrlLogDetail::Full,
        response_body: WpResponseBodyLogDetail::Full,
    };

    fn wp_error() -> WpApiError {
        WpApiError::WpError {
            error_code: WpErrorCode::Unauthorized,
            error_message: format!("{PERSONAL_DATA} is not authorized"),
            status_code: 401,
            response: ERROR_BODY.to_string(),
            request_url: TOKEN_INFO_URL.to_string(),
            request_method: RequestMethod::GET,
        }
    }

    fn unknown_error() -> WpApiError {
        WpApiError::UnknownError {
            status_code: 400,
            response: ERROR_BODY.to_string(),
            request_url: TOKEN_INFO_URL.to_string(),
            request_method: RequestMethod::GET,
        }
    }

    #[test]
    fn the_default_policy_keeps_query_values_but_never_a_credential() {
        // The default is security-oriented: it describes the request as fully
        // as it can without recording something that grants access.
        let described = wp_api_error_log_description(&wp_error(), DEFAULT_POLICY);
        assert!(described.contains("client_id=11"), "{described}");
        assert!(described.contains("token=REDACTED"), "{described}");
        assert!(!described.contains(ACCESS_TOKEN), "{described}");
    }

    #[test]
    fn the_privacy_policy_drops_query_values_as_well() {
        let described = wp_api_error_log_description(&wp_error(), PRIVATE_POLICY);
        assert!(!described.contains("client_id=11"), "{described}");
        assert!(!described.contains(ACCESS_TOKEN), "{described}");
    }

    #[test]
    fn the_default_policy_keeps_the_message_a_wp_error_carries() {
        // The server's account of the failure is what makes a report
        // actionable, and it cannot carry a credential the request sent.
        let described = wp_api_error_log_description(&wp_error(), DEFAULT_POLICY);
        assert!(
            described.contains(&format!("message={PERSONAL_DATA} is not authorized")),
            "{described}"
        );
        assert!(described.contains("code=Unauthorized"), "{described}");
        assert!(described.contains("status=401"), "{described}");
    }

    #[test]
    fn an_omitted_response_withholds_the_message_a_wp_error_carries() {
        let described = wp_api_error_log_description(&wp_error(), PRIVATE_POLICY);
        assert!(!described.contains(PERSONAL_DATA), "{described}");
        assert!(!described.contains("message="), "{described}");
    }

    #[test]
    fn the_default_policy_summarizes_the_response_body_instead_of_quoting_it() {
        let described = wp_api_error_log_description(&unknown_error(), DEFAULT_POLICY);
        assert!(described.contains("response=<"), "{described}");
        assert!(!described.contains(PERSONAL_DATA), "{described}");
    }

    #[test]
    fn an_omitted_body_leaves_no_response_field_at_all() {
        let described = wp_api_error_log_description(&unknown_error(), STRICT_POLICY);
        assert!(!described.contains("response="), "{described}");
    }

    #[test]
    fn the_default_policy_never_quotes_the_body_itself() {
        // The body is the one part of a response no policy short of `Full`
        // reports, however useful the message it wraps.
        let described = wp_api_error_log_description(&unknown_error(), DEFAULT_POLICY);
        assert!(described.contains("response=<"), "{described}");
        assert!(
            !described.contains(r#"{"code":"invalid_token""#),
            "{described}"
        );
    }

    #[test]
    fn a_path_only_url_leaves_no_query_string_at_all() {
        let described = wp_api_error_log_description(&unknown_error(), STRICT_POLICY);
        assert!(
            described.contains("url=https://public-api.wordpress.com/oauth2/token-info"),
            "{described}"
        );
        assert!(!described.contains("client_id"), "{described}");
    }

    #[test]
    fn a_full_policy_quotes_the_url_and_body_minus_the_always_redacted_parameters() {
        let described = wp_api_error_log_description(&unknown_error(), FULL_POLICY);
        assert!(described.contains("client_id=11"), "{described}");
        assert!(
            described.contains(&format!("response={ERROR_BODY}")),
            "{described}"
        );
        assert!(!described.contains(ACCESS_TOKEN), "{described}");
    }

    #[test]
    fn the_reason_a_response_failed_to_parse_follows_the_response_policy() {
        // serde quotes the offending value, so the reason can name a user. It
        // is still the only field that says *why* the parse failed, so the
        // default keeps it and the privacy-oriented policy drops it.
        let error = WpApiError::ResponseParsingError {
            reason: format!(r#"invalid type: string "{PERSONAL_DATA}", expected u64"#),
            response: ERROR_BODY.to_string(),
            request_url: TOKEN_INFO_URL.to_string(),
            request_method: RequestMethod::GET,
        };

        let described = wp_api_error_log_description(&error, DEFAULT_POLICY);
        assert!(described.contains("reason=invalid type"), "{described}");

        let withheld = wp_api_error_log_description(&error, PRIVATE_POLICY);
        assert!(!withheld.contains(PERSONAL_DATA), "{withheld}");
        assert!(!withheld.contains("reason="), "{withheld}");
    }

    #[test]
    fn a_request_execution_failure_names_the_host_not_the_whole_url() {
        // Build the reason the way production does, from a response whose URL
        // carries a credential. Handing it a hostname already known to be clean
        // would assert nothing: the reason is the field that can reintroduce a
        // URL the policy has already reduced.
        let response = WpNetworkResponse {
            status_code: 403,
            body: b"<html>Forbidden</html>".to_vec(),
            response_header_map: Arc::new(WpNetworkHeaderMap::default()),
            request_url: WpEndpointUrl(TOKEN_INFO_URL.to_string()),
            request_method: RequestMethod::GET,
            request_header_map: Arc::new(WpNetworkHeaderMap::default()),
        };
        let reason = RequestExecutionErrorReason::try_from_response(&response)
            .expect("a 403 whose body is not a WpError is an execution failure");

        let error = WpApiError::RequestExecutionFailed {
            status_code: Some(403),
            redirects: None,
            reason,
            request_url: TOKEN_INFO_URL.to_string(),
            request_method: RequestMethod::GET,
        };

        let described = wp_api_error_log_description(&error, STRICT_POLICY);
        assert!(!described.contains(ACCESS_TOKEN), "{described}");
        assert!(!described.contains("client_id"), "{described}");
    }

    /// Each arm renders, names itself, and keeps a planted credential out of
    /// the line at every policy.
    #[rstest]
    #[case::invalid_status(
        WpApiError::InvalidHttpStatusCode {
            status_code: 999,
            request_url: TOKEN_INFO_URL.to_string(),
            request_method: RequestMethod::GET,
        },
        "InvalidHttpStatusCode"
    )]
    #[case::site_url_parsing(
        WpApiError::SiteUrlParsingError { reason: "empty host".to_string() },
        "SiteUrlParsingError"
    )]
    #[case::media_unreadable(
        WpApiError::MediaFileUnreadable { file_path: "/tmp/a.jpg".to_string() },
        "MediaFileUnreadable"
    )]
    fn every_error_arm_names_itself_and_leaks_nothing(
        #[case] error: WpApiError,
        #[case] expected_name: &str,
    ) {
        for policy in [DEFAULT_POLICY, PRIVATE_POLICY, FULL_POLICY] {
            let described = wp_api_error_log_description(&error, policy);
            assert!(described.starts_with(expected_name), "{described}");
            assert!(!described.contains(ACCESS_TOKEN), "{described}");
        }
    }

    /// The discovery arms, including the two that reach
    /// `request_execution_error_description` — the only path to it, and the one
    /// this branch added logging for.
    #[rstest]
    #[case::fetch_homepage(
        FindApiRootFailure::FetchHomepage {
            error: RequestExecutionError::RequestExecutionFailed {
                status_code: None,
                redirects: None,
                reason: RequestExecutionErrorReason::HttpTimeoutError,
                request_url: TOKEN_INFO_URL.to_string(),
                request_method: RequestMethod::GET,
            },
        },
        "FetchHomepage"
    )]
    #[case::rest_api_disabled(FindApiRootFailure::RestApiDisabled, "RestApiDisabled")]
    #[case::not_wordpress(
        FindApiRootFailure::ProbablyNotAWordPressSite,
        "ProbablyNotAWordPressSite"
    )]
    fn every_find_api_root_arm_names_itself_and_leaks_nothing(
        #[case] find_api_root_failure: FindApiRootFailure,
        #[case] expected_name: &str,
    ) {
        let failure = AutoDiscoveryAttemptFailure::FindApiRoot {
            parsed_site_url: Arc::new(ParsedUrl::parse("https://example.com/").expect("valid URL")),
            find_api_root_failure,
        };

        for policy in [DEFAULT_POLICY, PRIVATE_POLICY, FULL_POLICY] {
            let described = auto_discovery_failure_log_description(&failure, policy);
            assert!(described.contains(expected_name), "{described}");
            assert!(!described.contains(ACCESS_TOKEN), "{described}");
        }
    }

    #[test]
    fn an_error_code_outside_the_enum_cannot_forge_a_second_log_line() {
        // `CustomError` holds the response's `code` field verbatim, and a
        // plugin sets that freely. It is reported at every policy, so it is the
        // one field a site could otherwise use to split the entry in two.
        let error = WpApiError::WpError {
            error_code: WpErrorCode::CustomError("a\nE/Forged: second".to_string()),
            error_message: "nope".to_string(),
            status_code: 400,
            response: ERROR_BODY.to_string(),
            request_url: TOKEN_INFO_URL.to_string(),
            request_method: RequestMethod::GET,
        };

        let described = wp_api_error_log_description(&error, PRIVATE_POLICY);
        assert!(!described.contains('\n'), "{described}");
        assert!(
            described.contains("CustomError(a\\nE/Forged"),
            "{described}"
        );
    }

    #[test]
    fn an_unparsable_site_url_names_only_the_parse_failure() {
        let failure = AutoDiscoveryAttemptFailure::ParseSiteUrl {
            error: ParseUrlError::EmptyHost,
        };
        assert_eq!(
            auto_discovery_failure_log_description(&failure, DEFAULT_POLICY),
            "ParseSiteUrl(reason=EmptyHost)"
        );
    }

    #[test]
    fn a_transport_failure_without_a_status_says_so() {
        let error = WpApiError::RequestExecutionFailed {
            status_code: None,
            redirects: None,
            reason: RequestExecutionErrorReason::HttpTimeoutError,
            request_url: "https://example.com/wp-json".to_string(),
            request_method: RequestMethod::GET,
        };

        assert!(
            wp_api_error_log_description(&error, DEFAULT_POLICY).contains("status=none"),
            "a missing status should read as `none`"
        );
    }

    #[rstest]
    #[case(DEFAULT_POLICY)]
    #[case(STRICT_POLICY)]
    fn a_media_file_path_is_logged_whatever_the_policy_says(
        #[case] policy: WpRequestErrorLogPolicy,
    ) {
        // The policy covers the URL and the response; a local file path comes
        // from neither. Pinned so the boundary is a decision, not a surprise.
        let error = WpApiError::MediaFileNotFound {
            file_path: "/storage/emulated/0/DCIM/Camera/holiday.jpg".to_string(),
        };
        assert_eq!(
            wp_api_error_log_description(&error, policy),
            "MediaFileNotFound(path=/storage/emulated/0/DCIM/Camera/holiday.jpg)"
        );
    }

    #[test]
    fn a_discovery_failure_redacts_the_site_url_the_user_typed() {
        // A self-hosted site URL can carry HTTP Basic credentials.
        let failure = AutoDiscoveryAttemptFailure::FindApiRoot {
            parsed_site_url: Arc::new(
                ParsedUrl::parse("https://admin:hunter2@example.com/").expect("valid URL"),
            ),
            find_api_root_failure: FindApiRootFailure::ProbablyNotAWordPressSite,
        };

        let described = auto_discovery_failure_log_description(&failure, DEFAULT_POLICY);
        assert!(!described.contains("hunter2"), "{described}");
        assert!(
            described.contains("ProbablyNotAWordPressSite"),
            "{described}"
        );
    }

    #[test]
    fn a_discovery_failure_summarizes_an_unreadable_api_root() {
        let failure = AutoDiscoveryAttemptFailure::FetchAndParseApiRoot {
            parsed_site_url: Arc::new(ParsedUrl::parse("https://example.com/").expect("valid URL")),
            api_root_url: Arc::new(
                ParsedUrl::parse("https://example.com/wp-json/").expect("valid URL"),
            ),
            fetch_and_parse_api_root_failure: FetchAndParseApiRootFailure::ParseApiRoot {
                parsing_error_message: format!(r#"invalid type: string "{PERSONAL_DATA}""#),
                response_body: ERROR_BODY.to_string(),
                response_body_type: ResponseBodyType::ValidJson,
                reason: None,
            },
        };

        let described = auto_discovery_failure_log_description(&failure, DEFAULT_POLICY);
        // The parser's account of the failure is reported; the body it choked
        // on is only described.
        assert!(
            described.contains("parsingError=invalid type"),
            "{described}"
        );
        assert!(described.contains("response=<"), "{described}");
        assert!(!described.contains(ERROR_BODY), "{described}");
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
