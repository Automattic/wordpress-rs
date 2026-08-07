package rs.wordpress.api.kotlin

import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.requestExecutionErrorReasonIsDeviceOffline
import uniffi.wp_api.requestExecutionErrorReasonIsSiteUnreachable

/**
 * Extension properties for classifying connectivity failures.
 *
 * The request executor maps the underlying platform errors onto
 * [RequestExecutionErrorReason.NonExistentSiteError] and
 * [RequestExecutionErrorReason.DeviceIsOfflineError]. These properties expose
 * that distinction without requiring callers to match the variants themselves.
 *
 * The reason is available as `reason` on `WpRequestResult.RequestExecutionFailed`
 * and on `WpApiException.RequestExecutionFailed`.
 */

/**
 * Whether the site could not be reached — most reliably, the host did not
 * resolve.
 *
 * Distinct from [isDeviceOffline]: this indicates a problem reaching *this
 * particular site*, not a loss of device connectivity.
 *
 * Note that a refused connection (the host resolves, but nothing is listening)
 * is reported here as an HTTP error rather than an unreachable site, whereas
 * the Swift executor reports it as unreachable. Only a DNS failure is treated
 * as an unreachable site by every executor. A malformed site URL never reaches
 * this predicate; it surfaces as `WpApiException.SiteUrlParsingException`.
 */
val RequestExecutionErrorReason.isSiteUnreachable: Boolean
    get() = requestExecutionErrorReasonIsSiteUnreachable(this)

/**
 * Whether the request failed because the device has no network connection.
 *
 * Distinct from [isSiteUnreachable]: the site itself may be perfectly healthy.
 */
val RequestExecutionErrorReason.isDeviceOffline: Boolean
    get() = requestExecutionErrorReasonIsDeviceOffline(this)
