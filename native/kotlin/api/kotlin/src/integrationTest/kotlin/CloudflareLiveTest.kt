package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assumptions.assumeTrue
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import uniffi.wp_api.CloudflareErrorKind
import uniffi.wp_api.MediaCreateParams
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.localizedDescription
import java.io.File
import java.io.RandomAccessFile
import java.net.URI
import java.util.Locale
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull

/**
 * Sends real requests through OkHttp to a Cloudflare zone set up to produce each error.
 * See `test-data/cloudflare-responses/README.md`.
 *
 * `WPRS_CLOUDFLARE_TEST_ZONE=wpmtcf.co ./gradlew :api:kotlin:integrationTest --tests '*CloudflareLiveTest*'`
 */
class CloudflareLiveTest {
    private val zone: String? = System.getenv("WPRS_CLOUDFLARE_TEST_ZONE")

    @BeforeEach
    fun requireZone() = assumeTrue(zone != null, "Needs WPRS_CLOUDFLARE_TEST_ZONE")

    private fun apiClient(host: String, apiRoot: String = "/wp-json") = WpApiClient(
        wpOrgSiteApiRootUrl = URI("https://$host.$zone$apiRoot").toURL(),
        authProvider = WpAuthenticationProvider.none(),
        requestExecutor = WpRequestExecutor(networkAvailabilityProvider = { true })
    )

    private fun cloudflareError(result: WpRequestResult<*>): RequestExecutionErrorReason.CloudflareError {
        assertIs<WpRequestResult.RequestExecutionFailed<*>>(result)
        val reason = result.reason
        assertIs<RequestExecutionErrorReason.CloudflareError>(reason)
        return reason
    }

    private suspend fun assertOriginFailure(host: String, code: UInt, kind: CloudflareErrorKind) {
        val result = apiClient(host).request { it.users().retrieveMeWithEditContext() }
        val error = cloudflareError(result)
        assertEquals(code, error.cloudflareCode)
        assertEquals(kind, error.kind)
        assertNotNull(error.rayId)
    }

    private suspend fun assertSecurityRule(path: String, kind: CloudflareErrorKind) {
        val result = apiClient("cf-rules", path).request { it.users().retrieveMeWithEditContext() }
        val error = cloudflareError(result)
        assertNull(error.cloudflareCode)
        assertEquals(kind, error.kind)
    }

    @Test
    fun `origin down`() = runTest { assertOriginFailure("cf-521", 521u, CloudflareErrorKind.ORIGIN_DOWN) }

    @Test
    fun `origin SSL handshake failure`() = runTest {
        assertOriginFailure("cf-525", 525u, CloudflareErrorKind.ORIGIN_SSL)
    }

    @Test
    fun `origin DNS error`() = runTest {
        assertOriginFailure("cf-1016", 1016u, CloudflareErrorKind.ORIGIN_UNREACHABLE)
    }

    @Test
    fun `WAF block page`() = runTest { assertSecurityRule("/cf-test/block", CloudflareErrorKind.ACCESS_DENIED) }

    @Test
    fun `managed challenge`() = runTest { assertSecurityRule("/cf-test/challenge", CloudflareErrorKind.CHALLENGE) }

    /**
     * What the login screen shows. Discovery sends no `Accept` header, so Cloudflare's answer here
     * isn't JSON.
     */
    private suspend fun assertLoginMessage(host: String, expected: String) {
        val result = WpLoginClient(emptyList(), { true }).apiDiscovery("https://$host.$zone")
        assertIs<ApiDiscoveryResult.Failure>(result)
        assertEquals(expected, result.failure.localizedDescription(listOf(Locale.US)))
    }

    @Test
    fun `login to a site behind a challenge`() = runTest {
        assertLoginMessage("cf-challenge-all", "Your site's Cloudflare bot protection is blocking the app.")
    }

    @Test
    fun `login to a site behind a WAF block`() = runTest {
        assertLoginMessage("cf-block-all", "A Cloudflare security rule on your site blocked this request.")
    }

    @Test
    fun `login to a site with an origin DNS error`() = runTest {
        assertLoginMessage("cf-1016", "Cloudflare can't reach your site's server.")
    }

    @Test
    fun `login to a site whose server is down`() = runTest {
        assertLoginMessage("cf-521", "Cloudflare reports that your site's server is down.")
    }

    /**
     * The CMM-2500 case: a 150 MB upload to a Free-plan zone, whose limit is 100 MB.
     *
     * Cloudflare usually answers 413, but sometimes 502 with no code, after the client has sent
     * the whole body. Either way it's Cloudflare's own page.
     */
    @Test
    fun `upload too large`() = runTest {
        val file = File.createTempFile("wprs-cloudflare-150mb", ".mp4")
        try {
            RandomAccessFile(file, "rw").use { it.setLength(150L * 1024 * 1024) }
            val result = apiClient("testing").request {
                it.media().create(MediaCreateParams(filePath = file.absolutePath))
            }
            val error = cloudflareError(result)
            assertNull(error.cloudflareCode)
            assertContains(listOf(CloudflareErrorKind.REQUEST_TOO_LARGE, CloudflareErrorKind.ORIGIN_ERROR), error.kind)
            assertNotNull(error.rayId)
        } finally {
            file.delete()
        }
    }
}
