package rs.wordpress.api.kotlin

/**
 * Receives a concise, log-only description of a failed request (the value of
 * [toLogErrorString]). Invoked only when a request fails — never on success.
 *
 * Implementations should forward the message to their platform logger or
 * crash-reporting breadcrumbs. Even under the strictest [policy] the message
 * describes a failure rather than explaining it, so it must NEVER be surfaced
 * to users.
 */
fun interface RequestErrorLogger {
    fun logError(message: String)

    /**
     * How much of each failed request reaches [logError]. Defaults to
     * [RequestErrorLogPolicy]'s own defaults, which write down no value the
     * request or the response carried.
     */
    val policy: RequestErrorLogPolicy
        get() = RequestErrorLogPolicy()

    companion object {
        /**
         * A logger that receives failed requests at [policy] rather than the
         * default level of detail — a login client, whose URLs carry
         * credentials, wants less; a client serving a screen that is hard to
         * debug may want more.
         */
        fun withPolicy(
            policy: RequestErrorLogPolicy,
            sink: RequestErrorLogger
        ): RequestErrorLogger = ConfiguredRequestErrorLogger(policy, sink)
    }
}

private class ConfiguredRequestErrorLogger(
    override val policy: RequestErrorLogPolicy,
    private val sink: RequestErrorLogger
) : RequestErrorLogger {
    override fun logError(message: String) = sink.logError(message)
}
