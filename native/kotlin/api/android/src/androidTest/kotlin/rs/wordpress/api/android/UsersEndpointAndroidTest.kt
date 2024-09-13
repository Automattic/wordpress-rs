package rs.wordpress.api.android

import kotlinx.coroutines.test.runTest
import org.junit.Test
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpAuthentication

class UsersEndpointAndroidTest {
    // https://developer.android.com/studio/run/emulator-networking
    private val siteUrl = "http://10.0.2.2"
    private val client = WpApiClient(ParsedUrl.parse(siteUrl), WpAuthentication.None)

    @Test
    fun testUserListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.users().listWithViewContext(params = UserListParams())
        }
        assert(result is WpRequestResult.WpRequestSuccess)
    }
}
