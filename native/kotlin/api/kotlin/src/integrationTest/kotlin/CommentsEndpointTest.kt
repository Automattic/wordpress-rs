package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions
import org.junit.jupiter.api.Test
import uniffi.wp_api.CommentListParams
import uniffi.wp_api.CommentRetrieveParams
import uniffi.wp_api.SparseCommentFieldWithEditContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertNotNull
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
    fun testRetrieveCommentRequest() = runTest {
        val comment = client.request { requestBuilder ->
            requestBuilder.comments().retrieveWithEditContext(1, CommentRetrieveParams())
        }.assertSuccessAndRetrieveData().data
        assertNotNull(comment)
    }

    @Test
    fun testFilterCommentListRequest() = runTest {
        val commentList = client.request { requestBuilder ->
            requestBuilder.comments().filterListWithEditContext(
                params = CommentListParams(),
                fields = listOf(
                    SparseCommentFieldWithEditContext.AUTHOR,
                    SparseCommentFieldWithEditContext.DATE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(commentList.isNotEmpty())
        assertNull(commentList.first().authorEmail)
    }

    @Test
    fun testFilterRetrieveCommentRequest() = runTest {
        val sparseComment = client.request { requestBuilder ->
            requestBuilder.comments().filterRetrieveWithEditContext(
                commentId = 1,
                params = CommentRetrieveParams(),
                fields = listOf(
                    SparseCommentFieldWithEditContext.AUTHOR,
                    SparseCommentFieldWithEditContext.CONTENT
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNotNull(sparseComment)
        Assertions.assertNull(sparseComment.id)
    }
}