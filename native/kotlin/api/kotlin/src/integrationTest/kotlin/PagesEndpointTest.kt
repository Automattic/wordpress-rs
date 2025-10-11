package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.PostListParams
import uniffi.wp_api.PostRetrieveParams
import uniffi.wp_api.SparseAnyPostFieldWithEditContext
import uniffi.wp_api.PostEndpointType
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class PagesEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val client = defaultApiClient()

    @Test
    fun testPageListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.posts().listWithEditContext(PostEndpointType.Pages, PostListParams())
        }.assertSuccessAndRetrieveData().data
        assert(postList.isNotEmpty())
    }

    @Test
    fun testRetrievePageRequest() = runTest {
        val post = client.request { requestBuilder ->
            requestBuilder.posts()
                .retrieveWithEditContext(PostEndpointType.Pages, 2, PostRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertNotNull(post)
    }

    @Test
    fun testFilterPageListRequest() = runTest {
        val pageList = client.request { requestBuilder ->
            requestBuilder.posts().filterListWithEditContext(
                PostEndpointType.Pages,
                PostListParams(),
                listOf(
                    SparseAnyPostFieldWithEditContext.TITLE,
                    SparseAnyPostFieldWithEditContext.CONTENT
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(pageList.isNotEmpty())
        assertNull(pageList.first().slug)
    }

    @Test
    fun testFilterRetrievePageRequest() = runTest {
        val sparsePage = client.request { requestBuilder ->
            requestBuilder.posts().filterRetrieveWithEditContext(
                PostEndpointType.Pages,
                2,
                PostRetrieveParams(),
                listOf(
                    SparseAnyPostFieldWithEditContext.TITLE,
                    SparseAnyPostFieldWithEditContext.CONTENT
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNotNull(sparsePage)
        assertNull(sparsePage.slug)
    }

    @Test
    fun testErrorPageListRequestInvalidPageNumber() = runTest {
        val params = PostListParams(page = 99999999u)
        val result =
            client.request { requestBuilder ->
                requestBuilder.posts().listWithEditContext(PostEndpointType.Pages, params)
            }
        assert(result.wpErrorCode() is WpErrorCode.PostInvalidPageNumber)
    }

    @Test
    fun testPageListPagination() = runTest {
        val firstPageResponse = client.request { requestBuilder ->
            requestBuilder.posts()
                .listWithEditContext(PostEndpointType.Pages, PostListParams(perPage = 1u))
        }.assertSuccessAndRetrieveData()
        assert(firstPageResponse.data.isNotEmpty())
        val nextPageResponse = client.request { requestBuilder ->
            requestBuilder.posts()
                .listWithEditContext(PostEndpointType.Pages, firstPageResponse.nextPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(nextPageResponse.data.isNotEmpty())
        val prevPageResponse = client.request { requestBuilder ->
            requestBuilder.posts()
                .listWithEditContext(PostEndpointType.Pages, nextPageResponse.prevPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(prevPageResponse.data.isNotEmpty())
    }

    @Test
    fun ensureDateGmtIsParsedCorrectly() = runTest {
        val page = client.request { requestBuilder ->
            requestBuilder.posts()
                .retrieveWithEditContext(PostEndpointType.Pages, 2, PostRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            testCredentials.firstPostDateGmt,
            TestCredentials.UTC_DATE_FORMAT.format(page.dateGmt)
        )
    }
}
