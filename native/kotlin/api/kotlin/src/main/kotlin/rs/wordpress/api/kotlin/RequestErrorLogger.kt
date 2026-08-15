package rs.wordpress.api.kotlin

/**
 * Receives a concise, log-only description of a failed request (the value of
 * [toLogErrorString]). Invoked only when a request fails — never on success.
 *
 * Implementations forward the message to their platform logger or
 * crash-reporting breadcrumbs, and decide through [policy] how much of the
 * failure the message describes. Even under the strictest policy the message
 * describes a failure rather than explaining it, so it must NEVER be surfaced
 * to users.
 *
 * [WpRequestErrorLogger] implements this over a lambda, with a policy that
 * writes down no value the request or the response carried.
 */
interface RequestErrorLogger {
    fun logError(message: String)

    /** How much of each failed request reaches [logError]. */
    val policy: RequestErrorLogPolicy
}
