package rs.wordpress.api.kotlin

import uniffi.wp_api.WpRequestErrorLogPolicy

/**
 * A [RequestErrorLogger] that hands each message to [sink], at [policy].
 *
 * The policy defaults to [DEFAULT_REQUEST_ERROR_LOG_POLICY], which keeps query
 * parameter values and the response's account of the failure but never a
 * credential, so the common case is just the sink:
 *
 * ```kotlin
 * WpRequestErrorLogger { message -> AppLog.e(AppLog.T.API, message) }
 * ```
 *
 * A client whose URLs hold credentials names a stricter policy, and one behind
 * a screen that is hard to debug can name a wider one:
 *
 * ```kotlin
 * WpRequestErrorLogger(
 *     WpRequestErrorLogPolicy(WpRequestUrlLogDetail.PATH_ONLY, WpResponseBodyLogDetail.OMITTED)
 * ) { message -> AppLog.e(AppLog.T.API, message) }
 * ```
 */
class WpRequestErrorLogger(
    override val policy: WpRequestErrorLogPolicy = DEFAULT_REQUEST_ERROR_LOG_POLICY,
    private val sink: (String) -> Unit
) : RequestErrorLogger {
    override fun logError(message: String) = sink(message)
}
