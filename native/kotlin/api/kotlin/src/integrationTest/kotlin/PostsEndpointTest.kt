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

class PostsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val client = defaultApiClient()

    @Test
    fun testPostListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.posts().listWithEditContext(PostEndpointType.Posts, PostListParams())
        }.assertSuccessAndRetrieveData().data
        assert(postList.isNotEmpty())
    }

    @Test
    fun testRetrievePostRequest() = runTest {
        val post = client.request { requestBuilder ->
            requestBuilder.posts()
                .retrieveWithEditContext(PostEndpointType.Posts, 1, PostRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertNotNull(post)
    }

    @Test
    fun testFilterPostListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.posts().filterListWithEditContext(
                PostEndpointType.Posts,
                PostListParams(),
                listOf(
                    SparseAnyPostFieldWithEditContext.TITLE,
                    SparseAnyPostFieldWithEditContext.CONTENT
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
                PostEndpointType.Posts,
                1,
                PostRetrieveParams(),
                listOf(
                    SparseAnyPostFieldWithEditContext.TITLE,
                    SparseAnyPostFieldWithEditContext.CONTENT
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
            client.request { requestBuilder ->
                requestBuilder.posts().listWithEditContext(PostEndpointType.Posts, params)
            }
        assert(result.wpErrorCode() is WpErrorCode.PostInvalidPageNumber)
    }

    @Test
    fun testPostListPagination() = runTest {
        val firstPageResponse = client.request { requestBuilder ->
            requestBuilder.posts()
                .listWithEditContext(PostEndpointType.Posts, PostListParams(perPage = 1u))
        }.assertSuccessAndRetrieveData()
        assert(firstPageResponse.data.isNotEmpty())
        val nextPageResponse = client.request { requestBuilder ->
            requestBuilder.posts()
                .listWithEditContext(PostEndpointType.Posts, firstPageResponse.nextPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(nextPageResponse.data.isNotEmpty())
        val prevPageResponse = client.request { requestBuilder ->
            requestBuilder.posts()
                .listWithEditContext(PostEndpointType.Posts, nextPageResponse.prevPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(prevPageResponse.data.isNotEmpty())
    }

    @Test
    fun ensureDateGmtIsParsedCorrectly() = runTest {
        val post = client.request { requestBuilder ->
            requestBuilder.posts()
                .retrieveWithEditContext(PostEndpointType.Posts, 1, PostRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            testCredentials.firstPostDateGmt,
            TestCredentials.UTC_DATE_FORMAT.format(post.dateGmt)
        )
    }
}
