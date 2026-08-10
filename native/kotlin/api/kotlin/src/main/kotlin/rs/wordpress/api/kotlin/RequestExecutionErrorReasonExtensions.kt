package rs.wordpress.api.kotlin

import uniffi.wp_api.RequestExecutionErrorReason
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
 * Whether the site could not be reached at all — either its host did not resolve
 * ([RequestExecutionErrorReason.NonExistentSiteError]) or the host resolved but
 * no connection could be established
 * ([RequestExecutionErrorReason.ConnectionError]).
 *
 * This is a portable signal: every executor classifies both failure modes the
 * same way. To tell them apart — a domain that doesn't resolve vs. a server
 * that's down — match the two variants directly.
 *
 * Distinct from [isDeviceOffline]: this indicates a problem reaching *this
 * particular site*, not a loss of device connectivity. A connect timeout is not
 * included (it stays [RequestExecutionErrorReason.HttpTimeoutError]); a malformed
 * site URL never reaches this predicate either — it surfaces as
 * `WpApiException.SiteUrlParsingException`.
 */
val RequestExecutionErrorReason.isSiteUnreachable: Boolean
    get() = requestExecutionErrorReasonIsSiteUnreachable(this)

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
