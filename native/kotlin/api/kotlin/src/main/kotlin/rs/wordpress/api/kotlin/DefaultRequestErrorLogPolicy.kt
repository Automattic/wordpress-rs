package rs.wordpress.api.kotlin

import uniffi.wp_api.WpRequestErrorLogPolicy
import uniffi.wp_api.WpRequestUrlLogDetail
import uniffi.wp_api.WpResponseBodyLogDetail

/**
 * The policy a [RequestErrorLogger] uses when it states none of its own.
 *
 * It describes a failure as fully as it can without recording anything that
 * grants access: query parameter values and the response's account of the
 * failure are kept, credentials and the response body are not. That trades
 * privacy for diagnosability — the line can name a user, through a `search`
 * term or an error message — on the grounds that a log a user cannot read is
 * worth less than one that says what went wrong.
 *
 * A client that should record no value at all pairs
 * [WpRequestUrlLogDetail.QUERY_KEYS_ONLY] with
 * [WpResponseBodyLogDetail.OMITTED] instead.
 *
 * See [WpRequestErrorLogPolicy] for what each axis governs, and for the fields
 * no policy reaches.
 */
val DEFAULT_REQUEST_ERROR_LOG_POLICY = WpRequestErrorLogPolicy(
    requestUrl = WpRequestUrlLogDetail.FULL,
    responseBody = WpResponseBodyLogDetail.SUMMARY
)
