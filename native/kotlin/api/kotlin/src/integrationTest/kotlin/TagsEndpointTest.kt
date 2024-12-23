package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.TagListParams
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class TagsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testTagListRequest() = runTest {
        val tagList = client.request { requestBuilder ->
            requestBuilder.tags().listWithEditContext(params = TagListParams())
        }.assertSuccessAndRetrieveData().data
        assert(tagList.isNotEmpty())
    }
}
