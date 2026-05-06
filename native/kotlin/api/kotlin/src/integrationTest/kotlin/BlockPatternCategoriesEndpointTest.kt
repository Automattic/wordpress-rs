package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test

class BlockPatternCategoriesEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testBlockPatternCategoryListRequest() = runTest {
        val categories = client.request { requestBuilder ->
            requestBuilder.blockPatternCategories().list()
        }.assertSuccessAndRetrieveData().data
        assert(categories.isNotEmpty())
    }
}
