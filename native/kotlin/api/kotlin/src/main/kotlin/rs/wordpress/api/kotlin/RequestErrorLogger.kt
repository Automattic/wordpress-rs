package rs.wordpress.api.kotlin

/**
 * Receives a concise, log-only description of a failed request (the value of
 * [toLogErrorString]). Invoked only when a request fails — never on success.
 *
 * Implementations should forward the message to their platform logger or
 * crash-reporting breadcrumbs. The message may contain raw response bodies and
 * request URLs, so it must NEVER be surfaced to users.
 */
fun interface RequestErrorLogger {
    fun logError(message: String)
}
