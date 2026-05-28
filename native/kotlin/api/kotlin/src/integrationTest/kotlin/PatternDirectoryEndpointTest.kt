package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PatternDirectoryListParams

class PatternDirectoryEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testPatternDirectoryListRequest() = runTest {
        val patterns = client.request { requestBuilder ->
            requestBuilder.patternDirectory()
                .listWithViewContext(PatternDirectoryListParams())
        }.assertSuccessAndRetrieveData().data
        assert(patterns.isNotEmpty())
    }

    @Test
    fun testPatternDirectoryListWithPerPage() = runTest {
        val patterns = client.request { requestBuilder ->
            requestBuilder.patternDirectory()
                .listWithViewContext(PatternDirectoryListParams(perPage = 3u))
        }.assertSuccessAndRetrieveData().data
        assert(patterns.size <= 3)
    }
}
