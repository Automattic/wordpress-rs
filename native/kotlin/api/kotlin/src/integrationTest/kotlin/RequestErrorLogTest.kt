package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpRequestErrorLogPolicy
import uniffi.wp_api.WpRequestUrlLogDetail
import uniffi.wp_api.WpResponseBodyLogDetail
import java.net.URI
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * Covers the binding between a client and its [RequestErrorLogger]: that a
 * failure reaches the logger exactly once, and at the logger's own policy.
 *
 * What the log line *says* is decided in Rust and covered by the
 * `log_redaction` tests in the `wp_api` crate.
 */
class RequestErrorLogTest {
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
    fun `a failed request reaches the logger once, at its default policy`() = runTest {
        mockWebServer.enqueue(MockResponse().setResponseCode(400).setBody(ERROR_BODY))

        val loggedErrors = mutableListOf<String>()
        val apiClient = apiClient(WpRequestErrorLogger { message -> loggedErrors.add(message) })

        apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams(perPage = 5u))
        }

        val message = loggedErrors.singleOrNull()
        assertNotNull(message, "expected exactly one logged failure: $loggedErrors")
        // The default policy keeps query values; only credentials are redacted.
        assertTrue(message.contains("per_page=5"), message)
    }

    @Test
    fun `a client logs through the policy its logger carries`() = runTest {
        mockWebServer.enqueue(MockResponse().setResponseCode(400).setBody(ERROR_BODY))

        val loggedErrors = mutableListOf<String>()
        val apiClient = apiClient(
            WpRequestErrorLogger(
                WpRequestErrorLogPolicy(
                    WpRequestUrlLogDetail.PATH_ONLY,
                    WpResponseBodyLogDetail.OMITTED
                )
            ) { message -> loggedErrors.add(message) }
        )

        apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams(perPage = 5u))
        }

        val message = loggedErrors.singleOrNull()
        assertNotNull(message, "expected exactly one logged failure: $loggedErrors")
        assertFalse(message.contains("per_page"), message)
        // The fixture is a `WpError`, which reports no `response=` at any
        // policy, so `message=` is what distinguishes OMITTED from the default.
        assertFalse(message.contains("message="), message)
    }

    @Test
    fun `a successful request logs nothing`() = runTest {
        mockWebServer.enqueue(MockResponse().setResponseCode(200).setBody("[]"))

        val loggedErrors = mutableListOf<String>()
        val apiClient = apiClient(WpRequestErrorLogger { message -> loggedErrors.add(message) })

        apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams())
        }

        assertEquals(emptyList(), loggedErrors)
    }

    @Test
    fun `a failed discovery reaches the login client's logger`() = runTest {
        mockWebServer.enqueue(MockResponse().setResponseCode(500).setBody("<html>nope</html>"))

        val loggedErrors = mutableListOf<String>()
        val loginClient = WpLoginClient(
            requestExecutor = executor(),
            errorLogger = WpRequestErrorLogger { message -> loggedErrors.add(message) }
        )

        // A self-hosted site URL a user types can carry HTTP Basic credentials.
        val siteUrl = mockWebServer.url("/").toString().replace("http://", "http://admin:hunter2@")
        loginClient.apiDiscovery(siteUrl)

        val message = loggedErrors.singleOrNull()
        assertNotNull(message, "expected exactly one logged failure: $loggedErrors")
        assertFalse(message.contains("hunter2"), message)
    }

    private fun executor() = WpRequestExecutor(
        httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
        networkAvailabilityProvider = NetworkAvailabilityProvider { true }
    )

    private fun apiClient(errorLogger: RequestErrorLogger) = WpApiClient(
        wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
        authProvider = WpAuthenticationProvider.none(),
        requestExecutor = executor(),
        errorLogger = errorLogger
    )

    companion object {
        private const val ERROR_BODY =
            """{"code":"invalid_token","message":"person@example.com is not authorized"}"""
    }
}
