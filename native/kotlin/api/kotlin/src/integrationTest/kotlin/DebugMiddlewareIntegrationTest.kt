package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.UniffiWpApiClient
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpApiClientDelegate
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpOrgSiteApiUrlResolver
import java.io.ByteArrayOutputStream
import java.io.PrintStream
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Runs [DebugMiddleware] on a real request, with the `Authorization` header the client adds, and
 * checks what it prints. The rules themselves are covered by the unit tests of the same name.
 */
class DebugMiddlewareIntegrationTest {
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
    fun `a request's credentials are sent but not printed`() = runTest {
        mockWebServer.enqueue(
            MockResponse()
                .setResponseCode(200)
                .addHeader("Set-Cookie", "session=$SECRET; HttpOnly")
                .setBody("[]")
        )

        val client = UniffiWpApiClient(
            WpOrgSiteApiUrlResolver(ParsedUrl.parse(mockWebServer.url("/wp-json").toString())),
            WpApiClientDelegate(
                WpAuthenticationProvider.staticWithAuth(WpAuthentication.Bearer(token = SECRET)),
                requestExecutor = WpRequestExecutor(
                    httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
                    networkAvailabilityProvider = NetworkAvailabilityProvider { true }
                ),
                middlewarePipeline = WpApiMiddlewarePipeline(listOf(DebugMiddleware())),
                appNotifier = EmptyAppNotifier()
            )
        )

        // The middleware prints to standard error, so that's where its output is read from.
        val captured = ByteArrayOutputStream()
        val originalErr = System.err
        System.setErr(PrintStream(captured, true, Charsets.UTF_8))
        try {
            client.users().listWithEditContext(params = UserListParams(perPage = 1u))
        } finally {
            System.setErr(originalErr)
        }

        // The header was on the request the middleware saw.
        assertEquals("Bearer $SECRET", mockWebServer.takeRequest().getHeader("Authorization"))

        val output = captured.toString(Charsets.UTF_8).lines().filter { it.startsWith("WpDebug:") }.joinToString("\n")
        assertFalse(output.contains(SECRET), output)
        assertTrue(output.contains(": Bearer <redacted>"), output)
        assertTrue(output.contains(": <redacted>"), output)
        assertTrue(output.contains("per_page=1"), output)
    }

    companion object {
        private const val SECRET = "not-a-real-token"
    }
}
