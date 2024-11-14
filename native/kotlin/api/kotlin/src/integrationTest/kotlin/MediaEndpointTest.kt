package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.MediaListParams
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class MediaEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testMediaListRequest() = runTest {
        val mediaList = client.request { requestBuilder ->
            requestBuilder.media().listWithEditContext(params = MediaListParams())
        }.assertSuccessAndRetrieveData().data
        assert(mediaList.isNotEmpty())
    }
}
