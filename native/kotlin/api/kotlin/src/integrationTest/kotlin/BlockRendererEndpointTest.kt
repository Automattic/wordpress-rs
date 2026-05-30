package rs.wordpress.api.kotlin

import kotlin.test.assertEquals

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.BlockRendererPostParams
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertTrue
class BlockRendererEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testRenderDynamicBlock() = runTest {
        val response = client.request { requestBuilder ->
            requestBuilder.blockRenderer().render(
                blockName = "core/latest-posts",
                params = BlockRendererPostParams()
            )
        }.assertSuccessAndRetrieveData().data
        assertTrue(response.rendered.isNotEmpty())
    }

    @Test
    fun testRenderDynamicBlockWithAttributes() = runTest {
        val response = client.request { requestBuilder ->
            requestBuilder.blockRenderer().render(
                blockName = "core/latest-posts",
                params = BlockRendererPostParams(
                    attributes = mapOf("postsToShow" to uniffi.wp_api.JsonValue.Int(1))
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertTrue(response.rendered.isNotEmpty())
    }

    @Test
    fun testRenderErrBlockInvalid() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.blockRenderer().render(
                blockName = "nonexistent/nonexistent",
                params = BlockRendererPostParams()
            )
        }
        assertEquals(WpErrorCode.BLOCK_INVALID, result.wpErrorCode())
    }

    @Test
    fun testRenderErrNonDynamicBlock() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.blockRenderer().render(
                blockName = "core/paragraph",
                params = BlockRendererPostParams()
            )
        }
        assertEquals(WpErrorCode.BLOCK_INVALID, result.wpErrorCode())
    }
}
