package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals

class BlockTypesEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testBlockTypeListRequest() = runTest {
        val blockTypes = client.request { requestBuilder ->
            requestBuilder.blockTypes().listWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assert(blockTypes.isNotEmpty())
    }

    @Test
    fun testBlockTypeListByNamespaceRequest() = runTest {
        val blockTypes = client.request { requestBuilder ->
            requestBuilder.blockTypes().listByNamespaceWithEditContext("core")
        }.assertSuccessAndRetrieveData().data
        assert(blockTypes.isNotEmpty())
    }

    @Test
    fun testBlockTypeRetrieveRequest() = runTest {
        val blockType = client.request { requestBuilder ->
            requestBuilder.blockTypes().retrieveWithEditContext("core", "paragraph")
        }.assertSuccessAndRetrieveData().data
        assertEquals("core/paragraph", blockType.name)
    }

    @Test
    fun testBlockTypeRetrieveErrInvalid() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.blockTypes().retrieveWithEditContext("nonexistent", "nonexistent")
        }
        assertEquals(WpErrorCode.BLOCK_TYPE_INVALID, result.wpErrorCode())
    }
}
