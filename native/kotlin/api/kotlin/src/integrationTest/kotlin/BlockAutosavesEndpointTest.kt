package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.BlockCreateParams
import uniffi.wp_api.SparseBlockRevisionFieldWithEditContext
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals

class BlockAutosavesEndpointTest {
    private val client = defaultApiClient()
    private val testCredentials = TestCredentials.INSTANCE

    @Test
    fun testBlockAutosaveListRequest() = runTest {
        val autosaves = client.request { requestBuilder ->
            requestBuilder.blockAutosaves().listWithEditContext(
                testCredentials.autosavedBlockId
            )
        }.assertSuccessAndRetrieveData().data
        assert(autosaves.isNotEmpty())
    }

    @Test
    fun testRetrieveBlockAutosaveRequest() = runTest {
        val autosave = client.request { requestBuilder ->
            requestBuilder.blockAutosaves().retrieveWithEditContext(
                testCredentials.autosavedBlockId,
                testCredentials.autosaveIdForAutosavedBlockId
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(testCredentials.autosaveIdForAutosavedBlockId, autosave.id)
    }

    @Test
    fun testFilterBlockAutosaveListRequest() = runTest {
        val autosaves = client.request { requestBuilder ->
            requestBuilder.blockAutosaves().filterListWithEditContext(
                testCredentials.autosavedBlockId,
                listOf(
                    SparseBlockRevisionFieldWithEditContext.TITLE,
                    SparseBlockRevisionFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(autosaves.isNotEmpty())
        autosaves.forEach { autosave ->
            assertNull(autosave.id)
            assertNull(autosave.author)
        }
    }

    @Test
    fun testErrorBlockAutosaveListRequestInvalidParent() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.blockAutosaves().listWithEditContext(99999999L)
        }
        assertEquals(WpErrorCode.POST_INVALID_PARENT, result.wpErrorCode())
    }
}
