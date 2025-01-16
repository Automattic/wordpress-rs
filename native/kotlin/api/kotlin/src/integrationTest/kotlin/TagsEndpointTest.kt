package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseTagFieldWithEditContext
import uniffi.wp_api.TagListParams
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertNotNull
import kotlin.test.assertNull

private const val TAG_ID_100: Long = 100

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

    @Test
    fun testFilterTagListRequest() = runTest {
        val tagList = client.request { requestBuilder ->
            requestBuilder.tags().filterListWithEditContext(
                params = TagListParams(),
                fields = listOf(
                    SparseTagFieldWithEditContext.NAME,
                    SparseTagFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(tagList.isNotEmpty())
        assertNull(tagList.first().description)
    }

    @Test
    fun testRetrieveMediaRequest() = runTest {
        val tag = client.request { requestBuilder ->
            requestBuilder.tags().retrieveWithEditContext(TAG_ID_100)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(tag)
    }

    @Test
    fun testFilterRetrieveTagRequest() = runTest {
        val tag = client.request { requestBuilder ->
            requestBuilder.tags().filterRetrieveWithEditContext(
                TAG_ID_100,
                fields = listOf(
                    SparseTagFieldWithEditContext.NAME,
                    SparseTagFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(tag.description)
    }
}
