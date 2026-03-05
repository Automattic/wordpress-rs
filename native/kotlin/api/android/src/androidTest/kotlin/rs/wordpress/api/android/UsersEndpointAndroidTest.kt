package rs.wordpress.api.android

import kotlinx.coroutines.test.runTest
import org.junit.Test
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpAuthenticationProvider
import java.net.URL

class UsersEndpointAndroidTest {
    // https://developer.android.com/studio/run/emulator-networking
    private val siteUrl = "http://10.0.2.2"
    private val client = WpApiClient(
        wpOrgSiteApiRootUrl = URL(siteUrl),
        authProvider = WpAuthenticationProvider.none(),
        interceptors = emptyList(),
        networkAvailabilityProvider = NetworkAvailabilityProvider { true }
    )

    @Test
    fun testUserListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.users().listWithViewContext(params = UserListParams())
        }
        assert(result is WpRequestResult.Success)
    }
}
