package rs.wordpress.api.kotlin

import uniffi.wp_api.VerifyIssuedApplicationPasswordException

/**
 * The outcome of [WpLoginClient.verifyIssuedApplicationPassword], in the style of
 * [ApiDiscoveryResult], so a failed check is a value to handle rather than a thrown exception.
 *
 * [Blocked] and [Other] pass the underlying exception straight through. Read its localized,
 * translated message with `localizedDescription()`, and the underlying request error from its
 * `error` property.
 */
sealed class VerifyIssuedApplicationPasswordResult {
    data object Verified : VerifyIssuedApplicationPasswordResult()

    /** The site's server does not pass the `Authorization` header to WordPress. */
    data class Blocked(
        val failure: VerifyIssuedApplicationPasswordException.AuthorizationHeaderBlocked
    ) : VerifyIssuedApplicationPasswordResult()

    /** Any other failure. It does not identify the cause, so callers continue as before. */
    data class Other(
        val failure: VerifyIssuedApplicationPasswordException.Other
    ) : VerifyIssuedApplicationPasswordResult()
}
