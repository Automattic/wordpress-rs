package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test

class BlockPatternsEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testBlockPatternListRequest() = runTest {
        val patterns = client.request { requestBuilder ->
            requestBuilder.blockPatterns().listWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assert(patterns.isNotEmpty())
    }
}
