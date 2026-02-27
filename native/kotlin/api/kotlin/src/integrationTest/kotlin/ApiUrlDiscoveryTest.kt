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
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.InvalidSslErrorReason
import uniffi.wp_api.ParseUrlException
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException
import kotlin.test.assertContains

@Execution(ExecutionMode.CONCURRENT)
class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient(emptyList())

    @Test
    fun testLocalSite() = runTest {
        assertEquals(
            "http://localhost/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("http://localhost")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 1
    fun testValidSiteWorksCorrectly() = runTest {
        val executor = MockRequestExecutor(
            listOf(
                Stub.forUrl(
                    "https://vanilla.wpmt.co/",
                    WpNetworkResponse.withApiRoot("https://vanilla.wpmt.co/wp-json/")
                ),
                Stub.forUrl(
                    "https://vanilla.wpmt.co/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/vanilla-api-root.json")
                ),
            )
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://vanilla.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
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
        // AutoStrippedHttps strips admin paths and creates an attempt for https://vanilla.wpmt.co
        // The UserInput attempts will fail (no stubs for wp-login.php / wp-admin URLs)
        // and the AutoStrippedHttps attempt will succeed.
        val executor = MockRequestExecutor(
            stubs = listOf(
                Stub.forUrl(
                    "https://vanilla.wpmt.co/",
                    WpNetworkResponse.withApiRoot("https://vanilla.wpmt.co/wp-json/")
                ),
                Stub.forUrl(
                    "https://vanilla.wpmt.co/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/vanilla-api-root.json")
                ),
            ),
            missingStubResponse = WpNetworkResponse.empty
        )

        val client = WpLoginClient(executor)

        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://vanilla.wpmt.co/wp-login.php")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )

        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://vanilla.wpmt.co/wp-admin")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 4
    fun testAutoHttpsSupport() = runTest {
        // Input is http://, AutoStrippedHttps creates https:// attempt which succeeds.
        // The http:// UserInput attempt will fail (no stubs for http://).
        val executor = MockRequestExecutor(
            stubs = listOf(
                Stub.forUrl(
                    "https://vanilla.wpmt.co/",
                    WpNetworkResponse.withApiRoot("https://vanilla.wpmt.co/wp-json/")
                ),
                Stub.forUrl(
                    "https://vanilla.wpmt.co/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/vanilla-api-root.json")
                ),
            ),
            missingStubResponse = WpNetworkResponse.empty
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("http://vanilla.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 5
    fun testHttpOnlySite() = runTest {
        // HTTP site with no application passwords auth URL.
        // The https:// AutoStrippedHttps attempt fails (no stubs).
        // The http:// UserInput attempt succeeds in finding the API root,
        // but the site has no auth URL and uses HTTP -> ApplicationPasswordsDisabledForHttpSite.
        val executor = MockRequestExecutor(
            stubs = listOf(
                Stub.forUrl(
                    "http://no-https.wpmt.co/",
                    WpNetworkResponse.withApiRoot("http://no-https.wpmt.co/wp-json/")
                ),
                Stub.forUrl(
                    "http://no-https.wpmt.co/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/http-only-api-root.json")
                ),
            ),
            missingStubResponse = WpNetworkResponse.empty
        )

        val client = WpLoginClient(executor)
        val reason = client.apiDiscovery("http://no-https.wpmt.co").assertFailureFetchAndParseApiRoot()
            .getApplicationPasswordsNotSupportedReason()
        assertInstanceOf(ApplicationPasswordsDisabledForHttpSite::class.java, reason)
    }

    @Test // Spec Example 6
    fun testHttpOnlySiteWithApplicationPasswordsEnabled() = runTest {
        // HTTP site that has application passwords enabled despite being HTTP.
        val executor = MockRequestExecutor(
            stubs = listOf(
                Stub.forUrl(
                    "http://no-https-with-application-passwords.wpmt.co/",
                    WpNetworkResponse.withApiRoot("http://no-https-with-application-passwords.wpmt.co/wp-json/")
                ),
                Stub.forUrl(
                    "http://no-https-with-application-passwords.wpmt.co/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/http-only-with-app-passwords-api-root.json")
                ),
            ),
            missingStubResponse = WpNetworkResponse.empty
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "http://no-https-with-application-passwords.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("http://no-https-with-application-passwords.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 7
    fun testAggressivelyCachedSiteWithNoLinkHeader() = runTest {
        // Homepage has no Link header but HTML body contains a <link> tag with the API root.
        val executor = MockRequestExecutor(
            listOf(
                Stub.forUrl(
                    "https://aggressive-caching.wpmt.co/",
                    WpNetworkResponse.htmlResponse("/login-mocks/homepage-with-link-tag.html")
                ),
                Stub.forUrl(
                    "https://aggressive-caching.wpmt.co/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/aggressive-caching-api-root.json")
                ),
            )
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://aggressive-caching.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
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
        // Homepage returns non-WordPress HTML. No Link header, no <link> tag.
        // Fallback to /wp-json/ gets empty response -> ProbablyNotAWordPressSite.
        val executor = MockRequestExecutor(
            stubs = listOf(
                Stub.forUrl(
                    "https://google.com/",
                    WpNetworkResponse.htmlResponse("/login-mocks/homepage-not-wordpress.html")
                ),
            ),
            missingStubResponse = WpNetworkResponse.empty
        )

        val client = WpLoginClient(executor)
        val reason = client.apiDiscovery("https://google.com").assertFailureFindApiRoot()
        assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, reason)
    }

    @Test // Spec Example 10
    fun testWordPressSubdirectoryWithLinkHeader() = runTest {
        // Homepage URL includes query params; the Link header points to the subdirectory wp-json.
        val executor = MockRequestExecutor(
            listOf(
                Stub.forUrl(
                    "https://subdirectory.wpmt.co/index.php?link_header=true",
                    WpNetworkResponse.withApiRoot("https://subdirectory.wpmt.co/wordpress/wp-json/")
                ),
                Stub.forUrl(
                    "https://subdirectory.wpmt.co/wordpress/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/subdirectory-api-root.json")
                ),
            )
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            client.apiDiscovery("https://subdirectory.wpmt.co/index.php?link_header=true")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 11
    fun testWordPressSubdirectoryWithLinkTag() = runTest {
        // Homepage has no Link header but HTML body has a <link> tag pointing to subdirectory wp-json.
        // Note: Url::parse adds a trailing slash, so "https://subdirectory.wpmt.co?link_tag=true"
        // becomes "https://subdirectory.wpmt.co/?link_tag=true".
        val executor = MockRequestExecutor(
            listOf(
                Stub.forUrl(
                    "https://subdirectory.wpmt.co/?link_tag=true",
                    WpNetworkResponse.htmlResponse("/login-mocks/homepage-with-subdirectory-link-tag.html")
                ),
                Stub.forUrl(
                    "https://subdirectory.wpmt.co/wordpress/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/subdirectory-api-root.json")
                ),
            )
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            client.apiDiscovery("https://subdirectory.wpmt.co?link_tag=true")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 12
    fun testWordPressSubdirectoryWithRedirect() = runTest {
        // In real life, this URL redirects to the WordPress subdirectory homepage.
        // The mock simulates the final response after redirect: homepage with Link header.
        val executor = MockRequestExecutor(
            listOf(
                Stub.forUrl(
                    "https://subdirectory.wpmt.co/index.php?redirect=true",
                    WpNetworkResponse.withApiRoot("https://subdirectory.wpmt.co/wordpress/wp-json/")
                ),
                Stub.forUrl(
                    "https://subdirectory.wpmt.co/wordpress/wp-json/",
                    WpNetworkResponse.jsonResponse("/login-mocks/subdirectory-api-root.json")
                ),
            )
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            client.apiDiscovery("https://subdirectory.wpmt.co/index.php?redirect=true")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 13 (with no credentials)
    fun testWordPressHttpBasicWithMissingCredentials() = runTest {
        // Homepage returns 401 with WWW-Authenticate header.
        // No auth credentials provided -> HttpAuthenticationRequiredError.
        val executor = MockRequestExecutor(
            stubs = listOf(
                Stub.forHost(
                    "basic-auth.wpmt.co",
                    WpNetworkResponse.responseWithStatus(
                        401u,
                        mapOf("WWW-Authenticate" to "Basic realm=\"Restricted\"")
                    )
                ),
            ),
        )

        val client = WpLoginClient(executor)
        val reason =
            client.apiDiscovery("https://basic-auth.wpmt.co").assertFailureFindApiRoot()
                .getRequestExecutionErrorReason()
        assertInstanceOf(
            RequestExecutionErrorReason.HttpAuthenticationRequiredError::class.java,
            reason
        )
    }

    @Test // Spec Example 13 (with invalid credentials)
    fun testWordPressHttpBasicWithInvalidCredentials() = runTest {
        // Homepage returns 401 with WWW-Authenticate header.
        // The ApiDiscoveryAuthenticationMiddleware adds auth and retries, but still gets 401.
        // With auth in request headers -> HttpAuthenticationRejectedError.
        val executor = MockRequestExecutor(
            listOf(
                Stub.forHost(
                    "basic-auth.wpmt.co",
                    WpNetworkResponse.responseWithStatus(
                        401u,
                        mapOf("WWW-Authenticate" to "Basic realm=\"Restricted\"")
                    )
                ),
            )
        )

        val invalid =
            ApiDiscoveryAuthenticationMiddleware(username = "invalid", password = "invalid")
        val client = WpLoginClient(
            executor, WpApiMiddlewarePipeline(middlewares = listOf(invalid))
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
        // Homepage returns 401 without auth, but succeeds with valid auth.
        // The middleware retries with credentials; the authenticated request succeeds.
        val executor = MockRequestExecutor(
            listOf(
                // Authenticated requests succeed (more specific stub first)
                Stub(
                    evaluator = { request ->
                        request.url() == "https://basic-auth.wpmt.co/" &&
                            request.headerMap().toMap().containsKey("authorization")
                    },
                    response = WpNetworkResponse.withApiRoot("https://basic-auth.wpmt.co/wp-json/")
                ),
                Stub(
                    evaluator = { request ->
                        request.url() == "https://basic-auth.wpmt.co/wp-json/" &&
                            request.headerMap().toMap().containsKey("authorization")
                    },
                    response = WpNetworkResponse.jsonResponse("/login-mocks/basic-auth-api-root.json")
                ),
                // Unauthenticated requests return 401
                Stub.forHost(
                    "basic-auth.wpmt.co",
                    WpNetworkResponse.responseWithStatus(
                        401u,
                        mapOf("WWW-Authenticate" to "Basic realm=\"Restricted\"")
                    )
                ),
            )
        )

        val valid = ApiDiscoveryAuthenticationMiddleware(
            username = "test@example.com",
            password = "str0ngp4ssw0rd!"
        )

        val client = WpLoginClient(
            executor, WpApiMiddlewarePipeline(middlewares = listOf(valid))
        )

        assertEquals(
            "https://basic-auth.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://basic-auth.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 14
    fun testWordPressCustomRestApiPrefix() = runTest {
        // Site uses a custom REST API prefix (not /wp-json/).
        // The Link header points to the custom API root URL.
        val executor = MockRequestExecutor(
            listOf(
                Stub.forUrl(
                    "https://custom-rest-prefix.wpmt.co/",
                    WpNetworkResponse.withApiRoot("https://custom-rest-prefix.wpmt.co/custom-api/")
                ),
                Stub.forUrl(
                    "https://custom-rest-prefix.wpmt.co/custom-api/",
                    WpNetworkResponse.jsonResponse("/login-mocks/custom-rest-prefix-api-root.json")
                ),
            )
        )

        val client = WpLoginClient(executor)
        assertEquals(
            "https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://custom-rest-prefix.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 15
    fun testWordPressHeavyRateLimiting() = runTest {
        assertEquals(
            "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://aggressive-rate-limiting.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
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
        val reason = loginClient.apiDiscovery("https://wordpress-1315525-4803651.cloudwaysapps.com")
            .assertFailureFindApiRoot().getRequestExecutionErrorReason()
        assertInstanceOf(RequestExecutionErrorReason.InvalidSslError::class.java, reason)

        val sslError = (reason as RequestExecutionErrorReason.InvalidSslError).reason
        assertInstanceOf(
            InvalidSslErrorReason.CertificateNotValidForName::class.java,
            sslError
        )

        val hostname = (sslError as InvalidSslErrorReason.CertificateNotValidForName).hostname
        val presentedHostnames = sslError.presentedHostnames

        assertEquals(hostname, "wordpress-1315525-4803651.cloudwaysapps.com")
        assertContains(presentedHostnames, "vanilla.wpmt.co")
    }

    @Test // Spec Example 17 (with exception)
    fun testInvalidHttpsWithExceptionWorks() = runTest {
        val httpClient = WpHttpClient.DefaultHttpClient(emptyList())
        val executor = WpRequestExecutor(httpClient)
        httpClient.addAllowedAlternativeNamesForHostname(
            "vanilla.wpmt.co",
            listOf("wordpress-1315525-4803651.cloudwaysapps.com")
        )

        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            WpLoginClient(requestExecutor = executor).apiDiscovery("https://wordpress-1315525-4803651.cloudwaysapps.com")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test
    fun testAllowedHostnamesDoesNotBreakValidSites() = runTest {
        val httpClient = WpHttpClient.DefaultHttpClient(emptyList())
        val executor = WpRequestExecutor(httpClient)
        val loginClient = WpLoginClient(requestExecutor = executor)

        // First, configure an allowed hostname override for a specific cert/hostname pair
        httpClient.addAllowedAlternativeNamesForHostname(
            "vanilla.wpmt.co",
            listOf("wordpress-1315525-4803651.cloudwaysapps.com")
        )

        // The override should work
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://wordpress-1315525-4803651.cloudwaysapps.com")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )

        // Other valid SSL sites should still work via fallback to default hostname verification.
        // google.com uses wildcard/SAN certificates which require proper OkHttp verification.
        val reason = loginClient.apiDiscovery("https://google.com").assertFailureFindApiRoot()
        assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, reason)
    }

    @Test
    fun testCustomOkHttpClient() = runTest {
        val executor =
            WpRequestExecutor(httpClient = WpHttpClient.CustomOkHttpClient(client = OkHttpClient()))
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            WpLoginClient(requestExecutor = executor).apiDiscovery("https://vanilla.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }
}

private fun ApiDiscoveryResult.assertSuccess(): AutoDiscoveryAttemptSuccess {
    assert(this is ApiDiscoveryResult.Success)
    return (this as ApiDiscoveryResult.Success).success
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
    }
}
