package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.WpRequestUrlLogDetail
import uniffi.wp_api.WpResponseBodyLogDetail
import java.net.URI
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * Covers how [toLogErrorString] applies a [RequestErrorLogPolicy], and that a
 * client logs through the policy its [RequestErrorLogger] carries. The
 * redaction rules themselves are covered by the `log_redaction` tests in the
 * `wp_api` crate.
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
    fun `the default policy keeps query keys and drops their values`() {
        val message = wpError(TOKEN_INFO_URL).toLogErrorString()

        assertNotNull(message)
        assertTrue(message.contains("url=$REDACTED_TOKEN_INFO_URL"), message)
        assertFalse(message.contains(ACCESS_TOKEN), message)
    }

    @Test
    fun `the default policy summarizes the response body instead of quoting it`() {
        val message = unknownError(TOKEN_INFO_URL, ERROR_BODY).toLogErrorString()

        assertNotNull(message)
        assertTrue(message.contains("response=<"), message)
        assertFalse(message.contains("person@example.com"), message)
    }

    @Test
    fun `an omitted body leaves no response field at all`() {
        val message = unknownError(TOKEN_INFO_URL, ERROR_BODY).toLogErrorString(
            RequestErrorLogPolicy(responseBody = WpResponseBodyLogDetail.OMITTED)
        )

        assertNotNull(message)
        assertFalse(message.contains("response="), message)
    }

    @Test
    fun `a path-only url leaves no query string at all`() {
        val message = unknownError(TOKEN_INFO_URL, ERROR_BODY).toLogErrorString(
            RequestErrorLogPolicy(requestUrl = WpRequestUrlLogDetail.PATH_ONLY)
        )

        assertNotNull(message)
        assertTrue(message.contains("url=https://public-api.wordpress.com/oauth2/token-info"), message)
        assertFalse(message.contains("client_id"), message)
    }

    @Test
    fun `a full policy quotes the url and the body, minus the always-redacted parameters`() {
        val message = unknownError(TOKEN_INFO_URL, ERROR_BODY).toLogErrorString(
            RequestErrorLogPolicy(WpRequestUrlLogDetail.FULL, WpResponseBodyLogDetail.FULL)
        )

        assertNotNull(message)
        assertTrue(message.contains("client_id=11"), message)
        assertTrue(message.contains("response=$ERROR_BODY"), message)
        // `token` is redacted whichever detail is chosen.
        assertFalse(message.contains(ACCESS_TOKEN), message)
    }

    @Test
    fun `a client logs through the policy its logger carries`() = runTest {
        mockWebServer.enqueue(MockResponse().setResponseCode(400).setBody(ERROR_BODY))

        val loggedErrors = mutableListOf<String>()
        val apiClient = WpApiClient(
            wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
            authProvider = WpAuthenticationProvider.none(),
            requestExecutor = WpRequestExecutor(
                httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
                networkAvailabilityProvider = NetworkAvailabilityProvider { true }
            ),
            errorLogger = RequestErrorLogger.withPolicy(
                RequestErrorLogPolicy(
                    WpRequestUrlLogDetail.PATH_ONLY,
                    WpResponseBodyLogDetail.OMITTED
                )
            ) { message -> loggedErrors.add(message) }
        )

        apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams(perPage = 5u))
        }

        assertEquals(1, loggedErrors.size, "expected exactly one logged failure: $loggedErrors")
        val message = loggedErrors.single()
        assertFalse(message.contains("per_page"), message)
        assertFalse(message.contains("response="), message)
    }

    private fun wpError(requestUrl: String) = WpRequestResult.WpError<Unit>(
        errorCode = WpErrorCode.Unauthorized(),
        errorMessage = "Sorry, you are not allowed to do that.",
        statusCode = 401u,
        response = ERROR_BODY,
        requestUrl = requestUrl,
        requestMethod = RequestMethod.GET
    )

    private fun unknownError(requestUrl: String, response: String) =
        WpRequestResult.UnknownError<Unit>(
            statusCode = 400u,
            response = response,
            requestUrl = requestUrl,
            requestMethod = RequestMethod.GET
        )

    companion object {
        private const val ACCESS_TOKEN = "s3cr3t-access-token"
        private const val TOKEN_INFO_URL =
            "https://public-api.wordpress.com/oauth2/token-info?client_id=11&token=$ACCESS_TOKEN"
        private const val REDACTED_TOKEN_INFO_URL =
            "https://public-api.wordpress.com/oauth2/token-info?client_id=REDACTED&token=REDACTED"
        private const val ERROR_BODY =
            """{"code":"invalid_token","message":"person@example.com is not authorized"}"""
    }
}
