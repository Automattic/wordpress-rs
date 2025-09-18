package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.PageListParams
import uniffi.wp_api.PageRetrieveParams
import uniffi.wp_api.SparsePageFieldWithEditContext
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class PagesEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val client = defaultApiClient()

    @Test
    fun testPageListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.pages().listWithEditContext(params = PageListParams())
        }.assertSuccessAndRetrieveData().data
        assert(postList.isNotEmpty())
    }

    @Test
    fun testRetrievePageRequest() = runTest {
        val post = client.request { requestBuilder ->
            requestBuilder.pages().retrieveWithEditContext(2, PageRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertNotNull(post)
    }

    @Test
    fun testFilterPageListRequest() = runTest {
        val pageList = client.request { requestBuilder ->
            requestBuilder.pages().filterListWithEditContext(
                params = PageListParams(),
                fields = listOf(
                    SparsePageFieldWithEditContext.TITLE,
                    SparsePageFieldWithEditContext.CONTENT
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(pageList.isNotEmpty())
        assertNull(pageList.first().slug)
    }

    @Test
    fun testFilterRetrievePageRequest() = runTest {
        val sparsePage = client.request { requestBuilder ->
            requestBuilder.pages().filterRetrieveWithEditContext(
                pageId = 2,
                params = PageRetrieveParams(),
                fields = listOf(
                    SparsePageFieldWithEditContext.TITLE,
                    SparsePageFieldWithEditContext.CONTENT
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNotNull(sparsePage)
        assertNull(sparsePage.slug)
    }

    @Test
    fun testErrorPageListRequestInvalidPageNumber() = runTest {
        val params = PageListParams(page = 99999999u)
        val result =
            client.request { requestBuilder -> requestBuilder.pages().listWithEditContext(params) }
        assert(result.wpErrorCode() is WpErrorCode.PostInvalidPageNumber)
    }

    @Test
    fun testPageListPagination() = runTest {
        val firstPageResponse = client.request { requestBuilder ->
            requestBuilder.pages().listWithEditContext(params = PageListParams(perPage = 1u))
        }.assertSuccessAndRetrieveData()
        assert(firstPageResponse.data.isNotEmpty())
        val nextPageResponse = client.request { requestBuilder ->
            requestBuilder.pages().listWithEditContext(firstPageResponse.nextPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(nextPageResponse.data.isNotEmpty())
        val prevPageResponse = client.request { requestBuilder ->
            requestBuilder.pages().listWithEditContext(nextPageResponse.prevPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(prevPageResponse.data.isNotEmpty())
    }

    @Test
    fun ensureDateGmtIsParsedCorrectly() = runTest {
        val page = client.request { requestBuilder ->
            requestBuilder.pages().retrieveWithEditContext(2, PageRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            testCredentials.firstPostDateGmt,
            TestCredentials.UTC_DATE_FORMAT.format(page.dateGmt)
        )
    }
}
