package rs.wordpress.api.kotlin

import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.AutoDiscoveryAttemptSuccess

/**
 * The outcome of [WpLoginClient.apiDiscovery]: a two-case envelope over the values the Rust
 * library emits, added only so a failed discovery is a value to handle rather than a thrown
 * exception a caller has to remember to catch.
 *
 * Both cases pass the underlying type straight through. Match [Failure.failure] — a sealed
 * [AutoDiscoveryAttemptFailure] — for the reason, and read its localized, translated message
 * with `localizedDescription()`.
 */
sealed class ApiDiscoveryResult {
    data class Success(val success: AutoDiscoveryAttemptSuccess) : ApiDiscoveryResult()
    data class Failure(val failure: AutoDiscoveryAttemptFailure) : ApiDiscoveryResult()
}
