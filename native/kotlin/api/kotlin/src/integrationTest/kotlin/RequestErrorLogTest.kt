package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import uniffi.wp_api.RequestExecutionErrorReason
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
    fun `the default policy withholds the message a WpError carries`() {
        // `errorMessage` is the `message` field lifted out of the response body,
        // so the body policy governs it too. The error code and status carry the
        // diagnosis without it.
        val message = wpError(TOKEN_INFO_URL).toLogErrorString()

        assertNotNull(message)
        assertFalse(message.contains(PERSONAL_DATA), message)
        assertFalse(message.contains("message="), message)
        assertTrue(message.contains("code=Unauthorized"), message)
        assertTrue(message.contains("status=401"), message)
    }

    @Test
    fun `a full policy restores the message a WpError carries`() {
        val message = wpError(TOKEN_INFO_URL).toLogErrorString(
            RequestErrorLogPolicy(WpRequestUrlLogDetail.FULL, WpResponseBodyLogDetail.FULL)
        )

        assertNotNull(message)
        assertTrue(message.contains("message=$ERROR_MESSAGE"), message)
    }

    @Test
    fun `the default policy withholds the reason a response failed to parse with`() {
        // serde quotes the offending value in its message, e.g.
        // `invalid type: string "person@example.com", expected u64`, so the
        // reason is body-derived and follows the body policy.
        val message = WpRequestResult.ResponseParsingError<Unit>(
            reason = """invalid type: string "$PERSONAL_DATA", expected u64""",
            response = ERROR_BODY,
            requestUrl = TOKEN_INFO_URL,
            requestMethod = RequestMethod.GET
        ).toLogErrorString()

        assertNotNull(message)
        assertFalse(message.contains(PERSONAL_DATA), message)
        assertFalse(message.contains("reason="), message)
        // The body's shape still says what failed to parse.
        assertTrue(message.contains("response=<"), message)
    }

    @Test
    fun `a request execution failure logs a hostname, not the whole url`() {
        // The reason carries a `hostname`; it must not smuggle the query string
        // back into a line whose `url=` field was redacted.
        val message = WpRequestResult.RequestExecutionFailed<Unit>(
            statusCode = 403u,
            redirects = null,
            reason = RequestExecutionErrorReason.HttpForbiddenError(
                hostname = "public-api.wordpress.com"
            ),
            requestUrl = TOKEN_INFO_URL,
            requestMethod = RequestMethod.GET
        ).toLogErrorString(
            RequestErrorLogPolicy(WpRequestUrlLogDetail.PATH_ONLY, WpResponseBodyLogDetail.OMITTED)
        )

        assertNotNull(message)
        assertFalse(message.contains(ACCESS_TOKEN), message)
        assertFalse(message.contains("client_id"), message)
    }

    @Test
    fun `a media file path is logged whatever the policy says`() {
        // The policy covers the URL and the response; a local file path comes
        // from neither. Pinned so the boundary is a decision, not a surprise.
        val message = WpRequestResult.MediaFileNotFound<Unit>(filePath = MEDIA_PATH).toLogErrorString(
            RequestErrorLogPolicy(WpRequestUrlLogDetail.PATH_ONLY, WpResponseBodyLogDetail.OMITTED)
        )

        assertEquals("MediaFileNotFound(path=$MEDIA_PATH)", message)
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
        errorMessage = ERROR_MESSAGE,
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
        private const val PERSONAL_DATA = "person@example.com"
        private const val TOKEN_INFO_URL =
            "https://public-api.wordpress.com/oauth2/token-info?client_id=11&token=$ACCESS_TOKEN"
        private const val REDACTED_TOKEN_INFO_URL =
            "https://public-api.wordpress.com/oauth2/token-info?client_id=REDACTED&token=REDACTED"
        private const val ERROR_MESSAGE = "$PERSONAL_DATA is not authorized"
        private const val ERROR_BODY = """{"code":"invalid_token","message":"$ERROR_MESSAGE"}"""
        private const val MEDIA_PATH = "/storage/emulated/0/DCIM/Camera/holiday.jpg"
    }
}
