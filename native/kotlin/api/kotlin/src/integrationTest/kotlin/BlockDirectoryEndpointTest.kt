package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.BlockDirectorySearchParams

class BlockDirectoryEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testBlockDirectorySearchRequest() = runTest {
        val results = client.request { requestBuilder ->
            requestBuilder.blockDirectory()
                .search(BlockDirectorySearchParams(term = "coblocks"))
        }.assertSuccessAndRetrieveData().data
        assert(results.isNotEmpty())
    }
}
