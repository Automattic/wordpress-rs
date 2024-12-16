package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.PostListParams
import uniffi.wp_api.PostRetrieveParams
import uniffi.wp_api.SparsePostFieldWithEditContext
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertNotNull

class PostsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testPostListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.posts().listWithEditContext(params = PostListParams())
        }.assertSuccessAndRetrieveData().data
        assert(postList.isNotEmpty())
    }

    @Test
    fun testRetrievePostRequest() = runTest {
        val post = client.request { requestBuilder ->
            requestBuilder.posts().retrieveWithEditContext(1, PostRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertNotNull(post)
    }

    @Test
    fun testFilterPostListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.posts().filterListWithEditContext(
                params = PostListParams(),
                fields = listOf(
                    SparsePostFieldWithEditContext.TITLE,
                    SparsePostFieldWithEditContext.CONTENT
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(postList.isNotEmpty())
        assertNull(postList.first().slug)
    }

    @Test
    fun testFilterRetrievePostRequest() = runTest {
        val sparsePost = client.request { requestBuilder ->
            requestBuilder.posts().filterRetrieveWithEditContext(
                postId = 1,
                params = PostRetrieveParams(),
                fields = listOf(
                    SparsePostFieldWithEditContext.TITLE,
                    SparsePostFieldWithEditContext.CONTENT
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNotNull(sparsePost)
        assertNull(sparsePost.slug)
    }

    @Test
    fun testErrorPostListRequestInvalidPageNumber() = runTest {
        val params = PostListParams(page = 99999999u)
        val result =
            client.request { requestBuilder -> requestBuilder.posts().listWithEditContext(params) }
        assert(result.wpErrorCode() is WpErrorCode.PostInvalidPageNumber)
    }

    @Test
    fun testPostListPagination() = runTest {
        val firstPageResponse = client.request { requestBuilder ->
            requestBuilder.posts().listWithEditContext(params = PostListParams(perPage = 1u))
        }.assertSuccessAndRetrieveData()
        assert(firstPageResponse.data.isNotEmpty())
        val nextPageResponse = client.request { requestBuilder ->
            requestBuilder.posts().listWithEditContext(firstPageResponse.nextPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(nextPageResponse.data.isNotEmpty())
        val prevPageResponse = client.request { requestBuilder ->
            requestBuilder.posts().listWithEditContext(nextPageResponse.prevPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(prevPageResponse.data.isNotEmpty())
    }
}
