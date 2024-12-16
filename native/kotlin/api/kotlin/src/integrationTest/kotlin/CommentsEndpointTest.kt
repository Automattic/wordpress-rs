package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions
import org.junit.jupiter.api.Test
import uniffi.wp_api.CommentCreateParams
import uniffi.wp_api.CommentDeleteParams
import uniffi.wp_api.CommentListParams
import uniffi.wp_api.CommentRetrieveParams
import uniffi.wp_api.CommentStatus
import uniffi.wp_api.CommentUpdateParams
import uniffi.wp_api.SparseCommentFieldWithEditContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals
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

    @Test
    fun createCommentRequest() = runTest {
        val createdComment = client.request { requestBuilder ->
            requestBuilder.comments()
                .create(CommentCreateParams(post = 1, content = "foo", status = CommentStatus.Hold))
        }.assertSuccessAndRetrieveData().data
        assertEquals("foo", createdComment.content.raw)
        assertEquals(CommentStatus.Hold, createdComment.status)
        restoreTestServer()
    }

    @Test
    fun deleteCommentRequest() = runTest {
        val deletedComment = client.request { requestBuilder ->
            requestBuilder.comments().delete(commentId = 1, CommentDeleteParams())
        }.assertSuccessAndRetrieveData().data
        assert(deletedComment.deleted)
        restoreTestServer()
    }

    @Test
    fun trashCommentRequest() = runTest {
        val trashedComment = client.request { requestBuilder ->
            requestBuilder.comments().trash(commentId = 1, CommentDeleteParams())
        }.assertSuccessAndRetrieveData().data
        assertEquals(CommentStatus.Trash, trashedComment.status)
        restoreTestServer()
    }

    @Test
    fun updateCommentRequest() = runTest {
        val updatedComment = client.request { requestBuilder ->
            requestBuilder.comments()
                .update(commentId = 1, CommentUpdateParams(content = "foo"))
        }.assertSuccessAndRetrieveData().data
        assertEquals("foo", updatedComment.content.raw)
        restoreTestServer()
    }
}
