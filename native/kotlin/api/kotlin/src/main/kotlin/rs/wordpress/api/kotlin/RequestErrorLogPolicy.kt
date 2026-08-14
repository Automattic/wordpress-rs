package rs.wordpress.api.kotlin

import uniffi.wp_api.WpRequestUrlLogDetail
import uniffi.wp_api.WpResponseBodyLogDetail

/**
 * How much of a failed request is written to the log line a
 * [RequestErrorLogger] receives.
 *
 * The defaults keep what identifies the request — status, error code, method,
 * endpoint, and which query parameters were sent — without writing down any
 * value the request or the response carried. Widen a field only for a client
 * whose traffic is known not to carry credentials or personal data.
 *
 * Some query parameters are redacted whichever [WpRequestUrlLogDetail] is
 * chosen; see `redact_request_url_for_log` in the `wp_api` crate for the list.
 */
data class RequestErrorLogPolicy(
    val requestUrl: WpRequestUrlLogDetail = WpRequestUrlLogDetail.QUERY_KEYS_ONLY,
    val responseBody: WpResponseBodyLogDetail = WpResponseBodyLogDetail.SUMMARY,
)
