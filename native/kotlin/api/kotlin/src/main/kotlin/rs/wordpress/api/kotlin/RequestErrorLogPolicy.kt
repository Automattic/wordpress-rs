package rs.wordpress.api.kotlin

import uniffi.wp_api.WpRequestUrlLogDetail
import uniffi.wp_api.WpResponseBodyLogDetail
import uniffi.wp_api.redactRequestUrlForLog
import uniffi.wp_api.redactResponseTextForLog
import uniffi.wp_api.summarizeResponseBodyForLog

/**
 * How much of a failed request is written to the log line a
 * [RequestErrorLogger] receives.
 *
 * The defaults keep what identifies the request — status, error code, method,
 * and endpoint — without writing down any value the request or the response
 * carried. Widen a field only for a client whose traffic is known not to carry
 * credentials or personal data.
 *
 * [requestUrl] governs the `url=` field. Some query parameters are redacted
 * whichever [WpRequestUrlLogDetail] is chosen; see `redactRequestUrlForLog` for
 * the list.
 *
 * [responseBody] governs every field taken from the failed response: the
 * `response=` body, the `message=` a `WpError` carries, and the `reason=` a
 * response failed to parse with. The latter two are free text with no shape to
 * summarize, so they appear only at [WpResponseBodyLogDetail.FULL].
 *
 * Two things no policy reaches, because they come from neither the URL nor the
 * response: the local file path on `MediaFileNotFound` and
 * `MediaFileUnreadable`, and the platform error text inside a
 * `RequestExecutionFailed` reason.
 */
data class RequestErrorLogPolicy(
    val requestUrl: WpRequestUrlLogDetail = WpRequestUrlLogDetail.QUERY_KEYS_ONLY,
    val responseBody: WpResponseBodyLogDetail = WpResponseBodyLogDetail.SUMMARY
) {
    companion object {
        /** Writes down no value the request or the response carried. */
        val DEFAULT = RequestErrorLogPolicy()
    }
}

/** A request URL reduced to what [RequestErrorLogPolicy.requestUrl] allows. */
internal fun RequestErrorLogPolicy.redactedUrl(url: String): String =
    redactRequestUrlForLog(url, requestUrl)

/**
 * The `response=` portion of a log line, or an empty string when the policy
 * leaves the body out entirely.
 */
internal fun RequestErrorLogPolicy.responseField(response: String): String =
    summarizeResponseBodyForLog(response, responseBody)?.let { ", response=$it" }.orEmpty()

/**
 * A `name=` portion of a log line carrying free text the failed response
 * supplied, or an empty string when the policy is not logging the body.
 */
internal fun RequestErrorLogPolicy.responseTextField(name: String, text: String): String =
    redactResponseTextForLog(text, responseBody)?.let { ", $name=$it" }.orEmpty()
