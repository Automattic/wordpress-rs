package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PostStatusSlug
import uniffi.wp_api.SparsePostStatusFieldWithViewContext
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

const val PUBLISH_STATUS_SLUG: String = "publish"
const val PUBLISH_STATUS_NAME: String = "published"

class PostStatusesEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testPostStatusesListRequest() = runTest {
        val postStatuses = client.request { requestBuilder ->
            requestBuilder.postStatuses().listWithEditContext()
        }.assertSuccessAndRetrieveData().data.postStatuses

        assertNotNull(postStatuses)
        assertTrue(postStatuses.isNotEmpty())

        // Verify that publish status exists and has expected properties
        val publishStatus = postStatuses[PostStatusSlug(PUBLISH_STATUS_SLUG)]
        assertNotNull(publishStatus)
        assertEquals(PUBLISH_STATUS_NAME, publishStatus.name)
        assertEquals(PUBLISH_STATUS_SLUG, publishStatus.slug)
    }

    @Test
    fun testPostStatusesListWithViewContext() = runTest {
        val postStatuses = client.request { requestBuilder ->
            requestBuilder.postStatuses().listWithViewContext()
        }.assertSuccessAndRetrieveData().data.postStatuses

        assertNotNull(postStatuses)
        assertTrue(postStatuses.isNotEmpty())

        // Verify publish status is accessible in view context
        val publishStatus = postStatuses[PostStatusSlug(PUBLISH_STATUS_SLUG)]
        assertNotNull(publishStatus)
        assertEquals(PUBLISH_STATUS_NAME, publishStatus.name)
    }

    @Test
    fun testPostStatusesRetrievePublish() = runTest {
        val publishStatus = client.request { requestBuilder ->
            requestBuilder.postStatuses()
                .retrieveWithEditContext(PostStatusSlug(PUBLISH_STATUS_SLUG))
        }.assertSuccessAndRetrieveData().data

        assertEquals(PUBLISH_STATUS_NAME, publishStatus.name)
        assertEquals(PUBLISH_STATUS_SLUG, publishStatus.slug)
    }

    @Test
    fun testPostStatusesErrStatusInvalid() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postStatuses()
                .retrieveWithViewContext(PostStatusSlug("non_existent_status"))
        }
        assert(result.wpErrorCode() is WpErrorCode.StatusInvalid)
    }

    @Test
    fun testPostStatusesErrCannotReadStatus() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postStatuses().retrieveWithViewContext(PostStatusSlug("auto-draft"))
        }
        assert(result.wpErrorCode() is WpErrorCode.CannotReadStatus)
    }

    @Test
    fun testPostStatusesFilterFields() = runTest {
        val publishStatus = client.request { requestBuilder ->
            requestBuilder.postStatuses().filterRetrieveWithViewContext(
                PostStatusSlug(PUBLISH_STATUS_SLUG),
                listOf(
                    SparsePostStatusFieldWithViewContext.NAME,
                    SparsePostStatusFieldWithViewContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data

        // Only requested fields should be present
        assertEquals(PUBLISH_STATUS_NAME, publishStatus.name)
        assertEquals(PUBLISH_STATUS_SLUG, publishStatus.slug)

        // Other fields should be null since they weren't requested
        assertEquals(null, publishStatus.public)
        assertEquals(null, publishStatus.queryable)
        assertEquals(null, publishStatus.dateFloating)
    }
}