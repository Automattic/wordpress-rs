package rs.wordpress.api.kotlin

import uniffi.wp_api.WpAppNotifier

class EmptyAppNotifier : WpAppNotifier {
    override suspend fun requestedWithInvalidAuthentication(requestUrl: String) {
        // no-op
    }
}
