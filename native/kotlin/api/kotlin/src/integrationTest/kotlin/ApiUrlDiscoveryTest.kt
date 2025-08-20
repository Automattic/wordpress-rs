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
    private val loginClient: WpLoginClient = WpLoginClient()

    @Test // Spec Example 1
    fun testValidSiteWorksCorrectly() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://vanilla.wpmt.co")
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
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://vanilla.wpmt.co/wp-login.php")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )

        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://vanilla.wpmt.co/wp-admin")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 4
    fun testAutoHttpsSupport() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("http://vanilla.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
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
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 7
    fun testAggressivelyCachedSiteWithNoLinkHeader() = runTest {
        assertEquals(
            "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://aggressive-caching.wpmt.co")
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
        val reason = loginClient.apiDiscovery("https://google.com").assertFailureFindApiRoot()
        assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, reason)
    }

    @Test // Spec Example 10
    fun testWordPressSubdirectoryWithLinkHeader() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://subdirectory.wpmt.co/index.php?link_header=true")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 11
    fun testWordPressSubdirectoryWithLinkTag() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://subdirectory.wpmt.co?link_tag=true")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 12
    fun testWordPressSubdirectoryWithRedirect() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://subdirectory.wpmt.co/index.php?redirect=true")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
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
            WpRequestExecutor(), WpApiMiddlewarePipeline(middlewares = listOf(invalid))
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
            WpRequestExecutor(), WpApiMiddlewarePipeline(middlewares = listOf(valid))
        )

        assertEquals(
            "https://basic-auth.wpmt.co/wp-admin/authorize-application.php",
            client.apiDiscovery("https://basic-auth.wpmt.co")
                .assertSuccess().applicationPasswordsAuthenticationUrl.url()
        )
    }

    @Test // Spec Example 14
    fun testWordPressCustomRestApiPrefix() = runTest {
        assertEquals(
            "https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php",
            loginClient.apiDiscovery("https://custom-rest-prefix.wpmt.co")
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
        val httpClient = WpHttpClient.DefaultHttpClient()
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
    }
}
