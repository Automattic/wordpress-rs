package rs.wordpress.api.kotlin

import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpRequestErrorLogPolicy
import uniffi.wp_api.autoDiscoveryFailureLogDescription
import uniffi.wp_api.wpApiErrorLogDescription

/**
 * Receives a concise, log-only description of a failed request. Invoked only
 * when a request fails — never on success.
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
    val policy: WpRequestErrorLogPolicy
}

/** Describes [exception] at this logger's policy and hands it to [RequestErrorLogger.logError]. */
internal fun RequestErrorLogger.logFailedRequest(exception: WpApiException) =
    logError(wpApiErrorLogDescription(exception, policy))

/** Describes [failure] at this logger's policy and hands it to [RequestErrorLogger.logError]. */
internal fun RequestErrorLogger.logFailedDiscovery(failure: AutoDiscoveryAttemptFailure) =
    logError(autoDiscoveryFailureLogDescription(failure, policy))
