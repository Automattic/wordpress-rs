package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import uniffi.wp_api.ApiDiscoveryAuthenticationMiddleware
import uniffi.wp_api.RetryAfterMiddleware
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpNetworkResponse
import kotlin.test.assertEquals
import org.junit.jupiter.api.Assertions.assertInstanceOf
import uniffi.wp_api.ApplicationPasswordsNotSupportedReason
import uniffi.wp_api.ApplicationPasswordsNotSupportedReason.ApplicationPasswordsDisabledForHttpSite
import uniffi.wp_api.ApplicationPasswordsNotSupportedReason.SiteIsLocalDevelopmentEnvironment
import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.applicationPasswordsUrl
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.InvalidSslErrorReason
import uniffi.wp_api.ParseUrlException
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException
import kotlin.test.assertContains
import kotlin.test.assertNotNull

@Execution(ExecutionMode.CONCURRENT)
class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient(emptyList(), NetworkAvailabilityProvider { true })

    @Test
    fun testLocalSite() = runTest {
        assertEquals(
            "http://localhost/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("http://localhost")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 1
    fun testValidSiteWorksCorrectly() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://vanilla.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 2
    fun testLocalDevelopmentEnvironment() = runTest {
        val executor = MockRequestExecutor(
            listOf(
                Stub.forUrl(
                    "http://localhost/",
                    WpNetworkResponse.withApiRoot("http://localhost/wp-json")
                ),
                Stub.forUrl(
                    "https://localhost/",
                    WpNetworkResponse.withApiRoot("http://localhost/wp-json")
                ),
                Stub.forUrl(
                    "http://localhost/wp-json",
                    WpNetworkResponse.jsonResponse("/localhost-json-root.json")
                ),
                Stub.forUrl(
                    "https://localhost/wp-json",
                    WpNetworkResponse.jsonResponse("/localhost-json-root.json")
                ),
            )
        )

        val client = WpLoginClient(executor)
        val reason = client.apiDiscovery("http://localhost").assertFailureFetchAndParseApiRoot()
            .getApplicationPasswordsNotSupportedReason()
        assertInstanceOf(SiteIsLocalDevelopmentEnvironment::class.java, reason)
    }

    @Test // Spec Example 3
    fun testAdminUrlProvided() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://vanilla.wpmt.co/wp-login.php")
                .assertSuccess().assertApplicationPasswordsUrl()
        )

        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://vanilla.wpmt.co/wp-admin")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 4
    fun testAutoHttpsSupport() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("http://vanilla.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 5
    fun testHttpOnlySite() = runTest {
        val reason = loginClient.apiDiscovery("http://no-https.wpmt.co").assertFailureFetchAndParseApiRoot()
            .getApplicationPasswordsNotSupportedReason()
        assertInstanceOf(ApplicationPasswordsDisabledForHttpSite::class.java, reason)
    }

    @Test // Spec Example 6
    fun testHttpOnlySiteWithApplicationPasswordsEnabled() = runTest {
        assertEquals(
            "http://no-https-with-application-passwords.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("http://no-https-with-application-passwords.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 7
    fun testAggressivelyCachedSiteWithNoLinkHeader() = runTest {
        assertEquals(
            "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://aggressive-caching.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 8
    fun testSiteWithApplicationPasswordsDisabledByWordFence() = runTest {
        val reason = loginClient.apiDiscovery("https://wordfence.wpmt.co")
            .assertFailureFetchAndParseApiRoot()
            .getApplicationPasswordsNotSupportedReason()
        assertInstanceOf(
            ApplicationPasswordsNotSupportedReason.ApplicationPasswordBlockedByPlugin::class.java,
            reason
        )
        val plugin =
            (reason as ApplicationPasswordsNotSupportedReason.ApplicationPasswordBlockedByPlugin).plugin
        assertEquals(plugin.name, "Wordfence")
    }

    @Test // Spec Example 9
    fun testNotWordPressSite() = runTest {
        val reason = loginClient.apiDiscovery("https://google.com").assertFailureFindApiRoot()
        assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, reason)
    }

    @Test // Spec Example 10
    fun testWordPressSubdirectoryWithLinkHeader() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://subdirectory.wpmt.co/index.php?link_header=true")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 11
    fun testWordPressSubdirectoryWithLinkTag() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://subdirectory.wpmt.co?link_tag=true")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 12
    fun testWordPressSubdirectoryWithRedirect() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://subdirectory.wpmt.co/index.php?redirect=true")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 13 (with no credentials)
    fun testWordPressHttpBasicWithMissingCredentials() = runTest {
        val reason =
            loginClient.apiDiscovery("https://basic-auth.wpmt.co").assertFailureFindApiRoot()
                .getRequestExecutionErrorReason()
        assertInstanceOf(
            RequestExecutionErrorReason.HttpAuthenticationRequiredError::class.java,
            reason
        )
    }

    @Test // Spec Example 13 (with invalid credentials)
    fun testWordPressHttpBasicWithInvalidCredentials() = runTest {
        val invalid =
            ApiDiscoveryAuthenticationMiddleware(username = "invalid", password = "invalid")
        val client = WpLoginClient(
            WpRequestExecutor(emptyList(), NetworkAvailabilityProvider { true }), WpApiMiddlewarePipeline(middlewares = listOf(invalid))
        )
        val reason = client.apiDiscovery("https://basic-auth.wpmt.co")
            .assertFailureFindApiRoot().getRequestExecutionErrorReason()
        assertInstanceOf(
            RequestExecutionErrorReason.HttpAuthenticationRejectedError::class.java,
            reason
        )
    }

    @Test // Spec Example 13 (with valid credentials)
    fun testWordPressHttpBasicWithValidCredentials() = runTest {
        val valid = ApiDiscoveryAuthenticationMiddleware(
            username = "test@example.com",
            password = "str0ngp4ssw0rd!"
        )

        val client = WpLoginClient(
            WpRequestExecutor(emptyList(), NetworkAvailabilityProvider { true }), WpApiMiddlewarePipeline(middlewares = listOf(valid))
        )

        assertEquals(
            "https://basic-auth.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://basic-auth.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 14
    fun testWordPressCustomRestApiPrefix() = runTest {
        assertEquals(
            "https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://custom-rest-prefix.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 15
    fun testWordPressHeavyRateLimiting() = runTest {
        assertEquals(
            "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://aggressive-rate-limiting.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }

    @Test // Spec Example 15
    fun testWordPressHeavyRateLimitingThatNeverSucceeds() = runTest {
        val executor = MockRequestExecutor(
            listOf(
                Stub.forHost(
                    "aggressive-rate-limiting.wpmt.co",
                    WpNetworkResponse.retryResponse(1u)
                )
            )
        )

        val middleware = RetryAfterMiddleware(maxRetries = 3u, maxRetryWaitSeconds = 1u)

        val client =
            WpLoginClient(executor, WpApiMiddlewarePipeline(middlewares = listOf(middleware)))
        val reason = client.apiDiscovery("https://aggressive-rate-limiting.wpmt.co")
            .assertFailureFindApiRoot().getRequestExecutionErrorReason()
        assertInstanceOf(
            RequestExecutionErrorReason.MisconfiguredRateLimitError::class.java,
            reason
        )
    }

    @Test // Spec Example 16
    fun testInvalidUrl() = runTest {
        val reason = loginClient.apiDiscovery("https://valid-looking-url-but-not-actually.foo")
            .assertFailureFindApiRoot().getRequestExecutionErrorReason()
        assertInstanceOf(RequestExecutionErrorReason.NonExistentSiteError::class.java, reason)
    }

    @Test // Spec Example 17
    fun testInvalidHTTPsFails() = runTest {
        // `wrong.host.badssl.com` serves a valid, trusted `*.badssl.com` certificate that doesn't
        // cover the host, so the chain is fine but the name doesn't match.
        val reason = loginClient.apiDiscovery("https://wrong.host.badssl.com")
            .assertFailureFindApiRoot().getRequestExecutionErrorReason()
        assertInstanceOf(RequestExecutionErrorReason.InvalidSslError::class.java, reason)

        val sslError = (reason as RequestExecutionErrorReason.InvalidSslError).reason
        assertInstanceOf(
            InvalidSslErrorReason.CertificateNotValidForName::class.java,
            sslError
        )

        val hostname = (sslError as InvalidSslErrorReason.CertificateNotValidForName).hostname
        val presentedHostnames = sslError.presentedHostnames

        assertEquals(hostname, "wrong.host.badssl.com")
        assertContains(presentedHostnames, "*.badssl.com")
    }

    // `wrong.host.badssl.com` serves a valid `*.badssl.com` certificate on a host
    // it doesn't cover — a genuine name mismatch whose identities live in the SANs.
    // The old code reported only the Common Name; assert the `badssl.com` SAN is
    // now included too.
    //
    // The CN-less case (`no-common-name.badssl.com`) isn't exercised here: that
    // certificate is expired, which OkHttp raises as an `SSLHandshakeException` —
    // not the `SSLPeerUnverifiedException` that routes to certificate inspection —
    // and the inspection's re-connect would fail on the expiry regardless. The
    // Rust `parse_certificate` unit test covers the CN-less parse directly.
    @Test
    fun testNameMismatchReportsAllPresentedNames() = runTest {
        val reason = loginClient.apiDiscovery("https://wrong.host.badssl.com")
            .assertFailureFindApiRoot().getRequestExecutionErrorReason()
        assertInstanceOf(RequestExecutionErrorReason.InvalidSslError::class.java, reason)

        val sslError = (reason as RequestExecutionErrorReason.InvalidSslError).reason
        assertInstanceOf(
            InvalidSslErrorReason.CertificateNotValidForName::class.java,
            sslError
        )

        val presentedHostnames =
            (sslError as InvalidSslErrorReason.CertificateNotValidForName).presentedHostnames
        assertContains(presentedHostnames, "badssl.com")
    }

    @Test // Spec Example 17 (with exception)
    fun testAllowedHostnamesDoesNotBreakValidSites() = runTest {
        val httpClient = WpHttpClient.DefaultHttpClient(emptyList())
        val executor = WpRequestExecutor(httpClient, NetworkAvailabilityProvider { true })
        val loginClient = WpLoginClient(requestExecutor = executor)

        // First, configure an allowed hostname override for a specific cert/hostname pair
        httpClient.addAllowedAlternativeNamesForHostname(
            "*.badssl.com",
            listOf("wrong.host.badssl.com")
        )

        // The override gets the mismatched host past the handshake; it then fails discovery only
        // because badssl.com isn't a WordPress site.
        val overrideReason = loginClient.apiDiscovery("https://wrong.host.badssl.com").assertFailureFindApiRoot()
        assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, overrideReason)

        // Other valid SSL sites should still work via fallback to default hostname verification.
        // google.com uses wildcard/SAN certificates which require proper OkHttp verification.
        val reason = loginClient.apiDiscovery("https://google.com").assertFailureFindApiRoot()
        assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, reason)
    }

    @Test // Alternative-name exception must not bypass chain validation
    fun testAllowedHostnamesStillValidatesCertificateChain() = runTest {
        val httpClient = WpHttpClient.DefaultHttpClient(emptyList())
        val executor = WpRequestExecutor(httpClient, NetworkAvailabilityProvider { true })
        httpClient.addAllowedAlternativeNamesForHostname(
            "*.badssl.com",
            listOf("self-signed.badssl.com")
        )

        // Allow-listing only relaxes the hostname check; the chain is still validated by the default
        // trust manager, so a self-signed certificate is rejected even for an allow-listed host.
        val reason = WpLoginClient(requestExecutor = executor)
            .apiDiscovery("https://self-signed.badssl.com")
            .assertFailureFindApiRoot().getRequestExecutionErrorReason()
        assertInstanceOf(RequestExecutionErrorReason.InvalidSslError::class.java, reason)
    }

    @Test // Layer 2: disabling validation accepts any certificate
    fun testDisableCertificateValidationWorks() = runTest {
        val httpClient = WpHttpClient.DefaultHttpClient(emptyList())
        val executor = WpRequestExecutor(httpClient, NetworkAvailabilityProvider { true })
        httpClient.disableCertificateValidation("self-signed.badssl.com")

        // With validation disabled for the host, even a self-signed certificate is accepted, so the
        // request gets past the handshake; discovery then fails only because badssl.com isn't a
        // WordPress site.
        val reason = WpLoginClient(requestExecutor = executor)
            .apiDiscovery("https://self-signed.badssl.com").assertFailureFindApiRoot()
        assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, reason)
    }

    @Test
    fun testCustomOkHttpClient() = runTest {
        val executor =
            WpRequestExecutor(httpClient = WpHttpClient.CustomOkHttpClient(client = OkHttpClient()), networkAvailabilityProvider = NetworkAvailabilityProvider { true })
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            WpLoginClient(requestExecutor = executor).apiDiscovery("https://vanilla.wpmt.co")
                .assertSuccess().assertApplicationPasswordsUrl()
        )
    }
}

private fun ApiDiscoveryResult.assertSuccess(): AutoDiscoveryAttemptSuccess {
    assert(this is ApiDiscoveryResult.Success)
    return (this as ApiDiscoveryResult.Success).success
}

private fun AutoDiscoveryAttemptSuccess.assertApplicationPasswordsUrl(): String {
    val parsedUrl = assertNotNull(
        uniffi.wp_api.applicationPasswordsUrl(this.authentication),
        "Expected application passwords authentication"
    )
    return parsedUrl.url()
}

private fun ApiDiscoveryResult.assertFailureParseSiteUrl(): ParseUrlException {
    assert(this is ApiDiscoveryResult.FailureParseSiteUrl)
    return (this as ApiDiscoveryResult.FailureParseSiteUrl).error
}

private fun ApiDiscoveryResult.assertFailureFindApiRoot(): FindApiRootFailure {
    assert(this is ApiDiscoveryResult.FailureFindApiRoot)
    return (this as ApiDiscoveryResult.FailureFindApiRoot).findApiRootFailure
}

private fun ApiDiscoveryResult.assertFailureFetchAndParseApiRoot(): FetchAndParseApiRootFailure {
    assert(this is ApiDiscoveryResult.FailureFetchAndParseApiRoot)
    return (this as ApiDiscoveryResult.FailureFetchAndParseApiRoot).fetchAndParseApiRootFailure
}

private fun FetchAndParseApiRootFailure.getApplicationPasswordsNotSupportedReason(): ApplicationPasswordsNotSupportedReason? {
    return when (this) {
        is FetchAndParseApiRootFailure.ApplicationPasswordsNotSupported -> this.reason
        else -> null
    }
}

private fun FindApiRootFailure.getRequestExecutionErrorReason(): RequestExecutionErrorReason? =
    when (this) {
        is FindApiRootFailure.FetchHomepage -> this.error.reason()
        else -> null
    }

private fun FetchAndParseApiRootFailure.getRequestExecutionErrorReason(): RequestExecutionErrorReason? =
    when (this) {
        is FetchAndParseApiRootFailure.FetchApiRoot -> this.error.reason()
        else -> null
    }

private fun RequestExecutionException.reason(): RequestExecutionErrorReason? {
    return when (this) {
        is RequestExecutionException.RequestExecutionFailed -> this.reason
        is RequestExecutionException.MediaFileNotFound -> null
        is RequestExecutionException.MediaFileUnreadable -> null
    }
}
