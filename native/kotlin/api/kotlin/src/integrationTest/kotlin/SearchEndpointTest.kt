package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SearchListParams
import uniffi.wp_api.SparseSearchResultFieldWithViewContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertNull

class SearchEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testSearchListRequest() = runTest {
        val list = client.request { requestBuilder ->
            requestBuilder.search().listWithViewContext(params = SearchListParams())
        }.assertSuccessAndRetrieveData().data
        assert(list.isNotEmpty())
    }

    @Test
    fun testFilterSearchListRequest() = runTest {
        val list = client.request { requestBuilder ->
            requestBuilder.search().filterListWithViewContext(
                params = SearchListParams(),
                fields = listOf(
                    SparseSearchResultFieldWithViewContext.ID,
                    SparseSearchResultFieldWithViewContext.OBJECT_TYPE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(list.isNotEmpty())
        assertNull(list.first().objectSubtype)
    }
}