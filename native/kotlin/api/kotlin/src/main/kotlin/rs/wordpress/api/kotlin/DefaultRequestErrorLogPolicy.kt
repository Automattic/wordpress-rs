package rs.wordpress.api.kotlin

import uniffi.wp_api.WpRequestErrorLogPolicy
import uniffi.wp_api.WpRequestUrlLogDetail
import uniffi.wp_api.WpResponseBodyLogDetail

/**
 * The policy a [RequestErrorLogger] uses when it states none of its own: keeps
 * what identifies the request, and writes down no value the request or the
 * response carried.
 *
 * See [WpRequestErrorLogPolicy] for what each axis governs, and for the fields
 * no policy reaches.
 */
val DEFAULT_REQUEST_ERROR_LOG_POLICY = WpRequestErrorLogPolicy(
    requestUrl = WpRequestUrlLogDetail.QUERY_KEYS_ONLY,
    responseBody = WpResponseBodyLogDetail.SUMMARY
)
