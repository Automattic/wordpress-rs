package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import kotlin.test.assertNotNull

class GlobalStylesEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val client = defaultApiClient()

    @Test
    fun testGlobalStylesRetrieveRequest() = runTest {
        val globalStyles = client.request { requestBuilder ->
            requestBuilder.globalStyles()
                .retrieveWithEditContext(testCredentials.globalStylesId)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(globalStyles)
    }
}
