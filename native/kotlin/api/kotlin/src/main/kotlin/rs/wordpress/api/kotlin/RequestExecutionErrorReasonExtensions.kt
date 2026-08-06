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
 * Whether the site could not be reached at all — the host did not resolve,
 * refused the connection, or the URL was malformed.
 *
 * Distinct from [isDeviceOffline]: this indicates a problem reaching *this
 * particular site*, not a loss of device connectivity.
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
