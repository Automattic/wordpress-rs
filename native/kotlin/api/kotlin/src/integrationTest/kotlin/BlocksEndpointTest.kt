package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.BlockListParams
import uniffi.wp_api.BlockRetrieveParams
import uniffi.wp_api.SparseBlockFieldWithEditContext
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals

class BlocksEndpointTest {
    private val client = defaultApiClient()
    private val testCredentials = TestCredentials.INSTANCE

    @Test
    fun testBlockListRequest() = runTest {
        val blocks = client.request { requestBuilder ->
            requestBuilder.blocks().listWithEditContext(BlockListParams())
        }.assertSuccessAndRetrieveData().data
        assert(blocks.isNotEmpty())
    }

    @Test
    fun testRetrieveBlockRequest() = runTest {
        val block = client.request { requestBuilder ->
            requestBuilder.blocks().retrieveWithEditContext(
                testCredentials.blockId,
                BlockRetrieveParams()
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(testCredentials.blockId, block.id)
    }

    @Test
    fun testFilterBlockListRequest() = runTest {
        val blocks = client.request { requestBuilder ->
            requestBuilder.blocks().filterListWithEditContext(
                BlockListParams(),
                listOf(
                    SparseBlockFieldWithEditContext.TITLE,
                    SparseBlockFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(blocks.isNotEmpty())
        blocks.forEach { block ->
            assertNull(block.id)
            assertNull(block.link)
        }
    }

    @Test
    fun testFilterRetrieveBlockRequest() = runTest {
        val block = client.request { requestBuilder ->
            requestBuilder.blocks().filterRetrieveWithEditContext(
                testCredentials.blockId,
                BlockRetrieveParams(),
                listOf(
                    SparseBlockFieldWithEditContext.TITLE,
                    SparseBlockFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(block.id)
        assertNull(block.link)
    }

    @Test
    fun testErrorBlockListRequestInvalidPageNumber() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.blocks().listWithEditContext(
                BlockListParams(page = 99999999u)
            )
        }
        assertEquals(WpErrorCode.POST_INVALID_PAGE_NUMBER, result.wpErrorCode())
    }
}
