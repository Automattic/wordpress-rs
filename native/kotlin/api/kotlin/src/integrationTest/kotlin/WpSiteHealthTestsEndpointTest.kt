package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseWpSiteHealthTestField

class WpSiteHealthTestsEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testBackgroundUpdates() = runTest {
        val wpSiteHealthTest = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().backgroundUpdates()
        }.assertSuccessAndRetrieveData().data
        assert(wpSiteHealthTest.test.isNotBlank())
    }

    @Test
    fun testLoopbackRequests() = runTest {
        val wpSiteHealthTest = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().loopbackRequests()
        }.assertSuccessAndRetrieveData().data
        assert(wpSiteHealthTest.test.isNotBlank())
    }

    @Test
    fun testHttpsStatus() = runTest {
        val wpSiteHealthTest = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().httpsStatus()
        }.assertSuccessAndRetrieveData().data
        assert(wpSiteHealthTest.test.isNotBlank())
    }

    @Test
    fun testDotOrgCommunication() = runTest {
        val wpSiteHealthTest = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().dotorgCommunication()
        }.assertSuccessAndRetrieveData().data
        assert(wpSiteHealthTest.test.isNotBlank())
    }

    @Test
    fun testAuthorizationHeader() = runTest {
        val wpSiteHealthTest = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().authorizationHeader()
        }.assertSuccessAndRetrieveData().data
        assert(wpSiteHealthTest.test.isNotBlank())
    }

    @Test
    fun testFilterBackgroundUpdates() = runTest {
        val wpSiteHealthTest = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests()
                .filterBackgroundUpdates(listOf(SparseWpSiteHealthTestField.TEST))
        }.assertSuccessAndRetrieveData().data
        assert(wpSiteHealthTest.test?.isBlank() == false)
    }

    @Test
    fun testDirectorySizes() = runTest {
        client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().directorySizes()
        }.assertSuccess()
    }
}
