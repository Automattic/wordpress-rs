package rs.wordpress.api.kotlin

import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.requestExecutionErrorReasonIsConnectivityFailure
import uniffi.wp_api.requestExecutionErrorReasonIsDeviceOffline
import uniffi.wp_api.requestExecutionErrorReasonIsSiteUnreachable

/**
 * Extension properties for classifying connectivity failures.
 *
 * The request executor maps the underlying platform errors onto
 * [RequestExecutionErrorReason.NonExistentSiteError],
 * [RequestExecutionErrorReason.ConnectionError], and
 * [RequestExecutionErrorReason.DeviceIsOfflineError]. These properties expose
 * those distinctions without requiring callers to match the variants themselves.
 *
 * The reason is available as `reason` on `WpRequestResult.RequestExecutionFailed`
 * and on `WpApiException.RequestExecutionFailed`.
 */

/**
 * Whether the site could not be reached because its host did not resolve — a
 * DNS-resolution failure ([RequestExecutionErrorReason.NonExistentSiteError]).
 *
 * Distinct from [isDeviceOffline]: this indicates a problem reaching *this
 * particular site*, not a loss of device connectivity.
 *
 * Note that a refused connection (the host resolves, but nothing is listening)
 * is classified as a [RequestExecutionErrorReason.ConnectionError] on every
 * executor, so it does not satisfy this predicate — use [isConnectivityFailure]
 * to cover both. A malformed site URL never reaches this predicate either; it
 * surfaces as `WpApiException.SiteUrlParsingException`.
 */
val RequestExecutionErrorReason.isSiteUnreachable: Boolean
    get() = requestExecutionErrorReasonIsSiteUnreachable(this)

/**
 * Whether the site's server could not be reached at all — either its host did
 * not resolve ([RequestExecutionErrorReason.NonExistentSiteError]) or the host
 * resolved but no connection could be established
 * ([RequestExecutionErrorReason.ConnectionError]).
 *
 * The broad counterpart to [isSiteUnreachable] (which is strictly the DNS case):
 * use this for a single "we couldn't reach your site" signal. Distinct from
 * [isDeviceOffline], the device's own loss of connectivity. A connect timeout is
 * not included (it stays [RequestExecutionErrorReason.HttpTimeoutError]).
 */
val RequestExecutionErrorReason.isConnectivityFailure: Boolean
    get() = requestExecutionErrorReasonIsConnectivityFailure(this)

/**
 * Whether the request failed because the device has no network connection.
 *
 * Distinct from [isSiteUnreachable]: the site itself may be perfectly healthy.
 *
 * This is only reported when the [NetworkAvailabilityProvider] supplied to the
 * executor reports the device offline at the moment a DNS lookup fails. It is
 * therefore best-effort: with no real provider wired in (the default reports the
 * device as always available) this never returns `true`, and a device that drops
 * offline mid-request — surfacing as a connect or timeout error rather than a DNS
 * failure — is not detected here either.
 */
val RequestExecutionErrorReason.isDeviceOffline: Boolean
    get() = requestExecutionErrorReasonIsDeviceOffline(this)
