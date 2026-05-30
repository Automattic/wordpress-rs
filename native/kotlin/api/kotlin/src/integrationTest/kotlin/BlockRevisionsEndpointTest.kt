package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.BlockRevisionListParams
import uniffi.wp_api.SparseBlockRevisionFieldWithEditContext
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals

class BlockRevisionsEndpointTest {
    private val client = defaultApiClient()
    private val testCredentials = TestCredentials.INSTANCE

    @Test
    fun testBlockRevisionListRequest() = runTest {
        val revisions = client.request { requestBuilder ->
            requestBuilder.blockRevisions().listWithEditContext(
                testCredentials.blockId,
                BlockRevisionListParams()
            )
        }.assertSuccessAndRetrieveData().data
        assert(revisions.isNotEmpty())
    }

    @Test
    fun testRetrieveBlockRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.blockRevisions().retrieveWithEditContext(
                testCredentials.blockId,
                testCredentials.revisionIdForBlockId
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(testCredentials.revisionIdForBlockId, revision.id)
    }

    @Test
    fun testFilterBlockRevisionListRequest() = runTest {
        val revisions = client.request { requestBuilder ->
            requestBuilder.blockRevisions().filterListWithEditContext(
                testCredentials.blockId,
                BlockRevisionListParams(),
                listOf(
                    SparseBlockRevisionFieldWithEditContext.TITLE,
                    SparseBlockRevisionFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(revisions.isNotEmpty())
        revisions.forEach { revision ->
            assertNull(revision.id)
            assertNull(revision.author)
        }
    }

    @Test
    fun testFilterRetrieveBlockRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.blockRevisions().filterRetrieveWithEditContext(
                testCredentials.blockId,
                testCredentials.revisionIdForBlockId,
                listOf(
                    SparseBlockRevisionFieldWithEditContext.TITLE,
                    SparseBlockRevisionFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(revision.id)
        assertNull(revision.author)
    }

    @Test
    fun testErrorBlockRevisionListRequestInvalidParent() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.blockRevisions().listWithEditContext(
                99999999L,
                BlockRevisionListParams()
            )
        }
        assertEquals(WpErrorCode.POST_INVALID_PARENT, result.wpErrorCode())
    }
}
