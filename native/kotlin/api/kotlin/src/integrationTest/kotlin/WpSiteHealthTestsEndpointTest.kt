package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseWpSiteHealthTestField
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class WpSiteHealthTestsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testBackgroundUpdates() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().backgroundUpdates()
        }
        assert(result is WpRequestSuccess)
        assert((result as WpRequestSuccess).data.test.isNotBlank())
    }

    @Test
    fun testLoopbackRequests() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().loopbackRequests()
        }
        assert(result is WpRequestSuccess)
        assert((result as WpRequestSuccess).data.test.isNotBlank())
    }

    @Test
    fun testHttpsStatus() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().httpsStatus()
        }
        assert(result is WpRequestSuccess)
        assert((result as WpRequestSuccess).data.test.isNotBlank())
    }

    @Test
    fun testDotOrgCommunication() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().dotorgCommunication()
        }
        assert(result is WpRequestSuccess)
        assert((result as WpRequestSuccess).data.test.isNotBlank())
    }

    @Test
    fun testAuthorizationHeader() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().authorizationHeader()
        }
        assert(result is WpRequestSuccess)
        assert((result as WpRequestSuccess).data.test.isNotBlank())
    }

    @Test
    fun testFilterBackgroundUpdates() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests()
                .filterBackgroundUpdates(listOf(SparseWpSiteHealthTestField.TEST))
        }
        assert(result is WpRequestSuccess)
        val wpSiteHealthTest = (result as WpRequestSuccess).data
        assert(wpSiteHealthTest.test?.isBlank() == false)
    }

    @Test
    fun testDirectorySizes() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.wpSiteHealthTests().directorySizes()
        }
        assert(result is WpRequestSuccess)
    }
}