package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import uniffi.wp_api.ApiDiscoveryAuthenticationMiddleware
import uniffi.wp_api.RetryAfterMiddleware
import uniffi.wp_api.WpApiMiddlewarePipeline
import uniffi.wp_api.WpNetworkResponse
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import org.junit.jupiter.api.Assertions.assertInstanceOf
import uniffi.wp_api.ApplicationPasswordsNotSupportedReason
import uniffi.wp_api.ApplicationPasswordsNotSupportedReason.ApplicationPasswordsDisabledForHttpSite
import uniffi.wp_api.ApplicationPasswordsNotSupportedReason.SiteIsLocalDevelopmentEnvironment
import uniffi.wp_api.AutoDiscoveryAttemptFailure
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException

@Execution(ExecutionMode.CONCURRENT)
class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient()

    @Test // Spec Example 1
    fun testValidSiteWorksCorrectly() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://vanilla.wpmt.co")
        )
    }

    @Test // Spec Example 2
    fun testLocalDevelopmentEnvironment() = runTest {
        val executor = MockRequestExecutor(listOf(
            Stub.forUrl("http://localhost/", WpNetworkResponse.withApiRoot("http://localhost/wp-json")),
            Stub.forUrl("https://localhost/", WpNetworkResponse.withApiRoot("http://localhost/wp-json")),
            Stub.forUrl("http://localhost/wp-json", WpNetworkResponse.jsonResponse("/localhost-json-root.json")),
            Stub.forUrl("https://localhost/wp-json", WpNetworkResponse.jsonResponse("/localhost-json-root.json")),
        ))

        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            val client = WpLoginClient(executor)
            client.loginUrl("http://localhost")
        }.let { e ->
            val reason = getApplicationPasswordsNotSupportedReason(e)
            assertInstanceOf(SiteIsLocalDevelopmentEnvironment::class.java, reason)
        }
    }

    @Test // Spec Example 3
    fun testAdminUrlProvided() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://vanilla.wpmt.co/wp-login.php")
        )

        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://vanilla.wpmt.co/wp-admin")
        )
    }

    @Test // Spec Example 4
    fun testAutoHttpsSupport() = runTest {
        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("http://vanilla.wpmt.co")
        )
    }

    @Test // Spec Example 5
    fun testHttpOnlySite() = runTest {
        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            loginClient.loginUrl("http://no-https.wpmt.co")
        }.let { e ->
            val reason = getApplicationPasswordsNotSupportedReason(e)
            assertInstanceOf(ApplicationPasswordsDisabledForHttpSite::class.java, reason)
        }
    }

    @Test // Spec Example 6
    fun testHttpOnlySiteWithApplicationPasswordsEnabled() = runTest {
        assertEquals(
            "http://no-https-with-application-passwords.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("http://no-https-with-application-passwords.wpmt.co")
        )
    }

    @Test // Spec Example 7
    fun testAggressivelyCachedSiteWithNoLinkHeader() = runTest {
        assertEquals(
            "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://aggressive-caching.wpmt.co")
        )
    }

    @Test // Spec Example 8
    fun testSiteWithApplicationPasswordsDisabledByWordFence() = runTest {
        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            loginClient.loginUrl("https://wordfence.wpmt.co")
        }.let { e ->
            val reason = getApplicationPasswordsNotSupportedReason(e)
            assertInstanceOf(ApplicationPasswordsNotSupportedReason.ApplicationPasswordBlockedByPlugin::class.java, reason)

            val plugin = (reason as ApplicationPasswordsNotSupportedReason.ApplicationPasswordBlockedByPlugin).plugin
            assertEquals(plugin.name, "Wordfence")
        }
    }

    @Test // Spec Example 9
    fun testNotWordPressSite() = runTest {
        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            loginClient.loginUrl("https://google.com")
        }.let { e ->
            val reason = getFindApiRootFailure(e)
            assertInstanceOf(FindApiRootFailure.ProbablyNotAWordPressSite::class.java, reason)
        }
    }

    @Test // Spec Example 10
    fun testWordPressSubdirectoryWithLinkHeader() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://subdirectory.wpmt.co/index.php?link_header=true")
        )
    }

    @Test // Spec Example 11
    fun testWordPressSubdirectoryWithLinkTag() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://subdirectory.wpmt.co?link_tag=true")
        )
    }

    @Test // Spec Example 12
    fun testWordPressSubdirectoryWithRedirect() = runTest {
        assertEquals(
            "https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://subdirectory.wpmt.co/index.php?redirect=true")
        )
    }

    @Test // Spec Example 13 (with no credentials)
    fun testWordPressHttpBasicWithMissingCredentials() = runTest {
        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            loginClient.loginUrl("https://basic-auth.wpmt.co")
        }.let { e ->
            val reason = getRequestExecutionErrorReason(e)
            assertInstanceOf(RequestExecutionErrorReason.HttpAuthenticationRequiredError::class.java, reason)
        }
    }

    @Test // Spec Example 13 (with invalid credentials)
    fun testWordPressHttpBasicWithInvalidCredentials() = runTest {
        val invalid = ApiDiscoveryAuthenticationMiddleware(username = "invalid", password = "invalid")
        val client = WpLoginClient(
            WpRequestExecutor(),
            WpApiMiddlewarePipeline(middlewares = listOf(invalid))
        )
        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            client.loginUrl("https://basic-auth.wpmt.co")
        }.let { e ->
            val reason = getRequestExecutionErrorReason(e)
            assertInstanceOf(RequestExecutionErrorReason.HttpAuthenticationRejectedError::class.java, reason)
        }
    }

    @Test // Spec Example 13 (with valid credentials)
    fun testWordPressHttpBasicWithValidCredentials() = runTest {
        val valid = ApiDiscoveryAuthenticationMiddleware(username = "test@example.com", password = "str0ngp4ssw0rd!")

        val client = WpLoginClient(
            WpRequestExecutor(),
            WpApiMiddlewarePipeline(middlewares = listOf(valid))
        )

        assertEquals(
            "https://basic-auth.wpmt.co/wp-admin/authorize-application.php",
            client.loginUrl("https://basic-auth.wpmt.co")
        )
    }

    @Test // Spec Example 14
    fun testWordPressCustomRestApiPrefix() = runTest {
        assertEquals(
            "https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://custom-rest-prefix.wpmt.co")
        )
    }

    @Test // Spec Example 15
    fun testWordPressHeavyRateLimiting() = runTest {
        assertEquals(
            "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://aggressive-rate-limiting.wpmt.co")
        )
    }

    @Test // Spec Example 15
    fun testWordPressHeavyRateLimitingThatNeverSucceeds() = runTest {
        val executor = MockRequestExecutor(listOf(
            Stub.forHost("aggressive-rate-limiting.wpmt.co", WpNetworkResponse.retryResponse(1u))
        ))

        val middleware = RetryAfterMiddleware(maxRetries = 3u, maxRetryWaitSeconds = 1u)

        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            val client = WpLoginClient(executor, WpApiMiddlewarePipeline(middlewares = listOf(middleware)))
            client.loginUrl("https://aggressive-rate-limiting.wpmt.co")
        }.let { e ->
            val reason = getRequestExecutionErrorReason(e)
            assertInstanceOf(RequestExecutionErrorReason.MisconfiguredRateLimitError::class.java, reason)
        }
    }

    @Test // Spec Example 16
    fun testInvalidUrl() = runTest {
        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            loginClient.loginUrl("https://valid-looking-url-but-not-actually.foo")
        }.let { e ->
            val reason = getRequestExecutionErrorReason(e)
            assertInstanceOf(RequestExecutionErrorReason.NonExistentSiteError::class.java, reason)
        }
    }

    @Test // Spec Example 17
    fun testInvalidHTTPsFails() = runTest {
        assertFailsWith<AutoDiscoveryAttemptFailure>() {
            loginClient.loginUrl("https://wordpress-1315525-4803651.cloudwaysapps.com")
        }.let { e ->
            val reason = getRequestExecutionErrorReason(e)
            assertInstanceOf(RequestExecutionErrorReason.InvalidSslError::class.java, reason)
        }
    }

    @Test // Spec Example 17 (with exception)
    fun testInvalidHttpsWithExceptionWorks() = runTest {
        val executor = WpRequestExecutor()
        executor.addAllowedAlternativeNameForHostname("wordpress-1315525-4803651.cloudwaysapps.com", "vanilla.wpmt.co")

        assertEquals(
            "https://vanilla.wpmt.co/wp-admin/authorize-application.php",
            WpLoginClient(requestExecutor = executor).loginUrl("https://wordpress-1315525-4803651.cloudwaysapps.com")
        )
    }

    private fun getApplicationPasswordsNotSupportedReason(error: AutoDiscoveryAttemptFailure): ApplicationPasswordsNotSupportedReason? {
        return when(val failure = getFetchAndParseApiRootFailure(error)) {
            is FetchAndParseApiRootFailure.ApplicationPasswordsNotSupported -> failure.reason
            else -> null
        }
    }

    private fun getFetchAndParseApiRootFailure(error: AutoDiscoveryAttemptFailure): FetchAndParseApiRootFailure? {
        return when(error) {
            is AutoDiscoveryAttemptFailure.FetchAndParseApiRoot -> error.fetchAndParseApiRootFailure
            else -> null
        }
    }

    private fun getFindApiRootFailure(error: AutoDiscoveryAttemptFailure): FindApiRootFailure? {
        return when (error) {
            is AutoDiscoveryAttemptFailure.FindApiRoot -> error.findApiRootFailure
            else -> null
        }
    }

    private fun getRequestExecutionErrorReason(error: AutoDiscoveryAttemptFailure): RequestExecutionErrorReason? {
        when(val failure = getFindApiRootFailure(error)) {
            is FindApiRootFailure.FetchHomepage -> return extract(failure.error)
            else -> {}
        }

        when(val failure = getFetchAndParseApiRootFailure(error)) {
            is FetchAndParseApiRootFailure.FetchApiRoot -> return extract(failure.error)
            else -> {}
        }

        return null
    }

    private fun extract(error: RequestExecutionException): RequestExecutionErrorReason? {
        return when(error) {
            is RequestExecutionException.RequestExecutionFailed -> error.reason
            else -> null
        }
    }

}
