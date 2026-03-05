package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.SocketPolicy
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpAuthenticationProvider
import java.net.URI
import java.util.concurrent.TimeUnit
import kotlin.test.assertIs

class WpRequestExecutorTest {
    private lateinit var mockWebServer: MockWebServer

    @BeforeEach
    fun setUp() {
        mockWebServer = MockWebServer()
        mockWebServer.start()
    }

    @AfterEach
    fun tearDown() {
        mockWebServer.shutdown()
    }

    @Test
    fun `timeout exception is mapped to HttpTimeoutError`() = runTest {
        // Configure the mock server to not respond (causing a timeout)
        mockWebServer.enqueue(
            MockResponse()
                .setSocketPolicy(SocketPolicy.NO_RESPONSE)
        )

        // Create an OkHttp client with a very short timeout
        val client = OkHttpClient.Builder()
            .connectTimeout(100, TimeUnit.MILLISECONDS)
            .readTimeout(100, TimeUnit.MILLISECONDS)
            .writeTimeout(100, TimeUnit.MILLISECONDS)
            .build()

        val executor = WpRequestExecutor(
            httpClient = WpHttpClient.CustomOkHttpClient(client),
            networkAvailabilityProvider = NetworkAvailabilityProvider { true }
        )

        val apiClient = WpApiClient(
            wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
            authProvider = WpAuthenticationProvider.none(),
            requestExecutor = executor
        )

        // Make a request that will timeout
        val result = apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams())
        }

        // Verify the error is categorized as HttpTimeoutError
        assertIs<WpRequestResult.RequestExecutionFailed<*>>(result)
        assertIs<RequestExecutionErrorReason.HttpTimeoutError>(
            (result as WpRequestResult.RequestExecutionFailed<*>).reason
        )
    }
}
