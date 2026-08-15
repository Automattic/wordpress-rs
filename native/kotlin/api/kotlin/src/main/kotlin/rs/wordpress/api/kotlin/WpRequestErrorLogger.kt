package rs.wordpress.api.kotlin

/**
 * A [RequestErrorLogger] that hands each message to [sink], at [policy].
 *
 * The policy defaults to writing down no value the request or the response
 * carried, so the common case is just the sink:
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
 *     RequestErrorLogPolicy(WpRequestUrlLogDetail.PATH_ONLY, WpResponseBodyLogDetail.OMITTED)
 * ) { message -> AppLog.e(AppLog.T.API, message) }
 * ```
 */
class WpRequestErrorLogger(
    override val policy: RequestErrorLogPolicy = RequestErrorLogPolicy.DEFAULT,
    private val sink: (String) -> Unit
) : RequestErrorLogger {
    override fun logError(message: String) = sink(message)
}
