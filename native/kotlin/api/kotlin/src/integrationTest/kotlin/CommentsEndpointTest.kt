package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.CommentListParams
import uniffi.wp_api.SparseCommentFieldWithEditContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertNull

class CommentsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testCommentListRequest() = runTest {
        val commentList = client.request { requestBuilder ->
            requestBuilder.comments().listWithEditContext(params = CommentListParams())
        }.assertSuccessAndRetrieveData().data
        assert(commentList.isNotEmpty())
    }

    @Test
    fun testFilterCommentListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.comments().filterListWithEditContext(
                params = CommentListParams(),
                fields = listOf(
                    SparseCommentFieldWithEditContext.AUTHOR,
                    SparseCommentFieldWithEditContext.DATE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(postList.isNotEmpty())
        assertNull(postList.first().authorEmail)
    }
}