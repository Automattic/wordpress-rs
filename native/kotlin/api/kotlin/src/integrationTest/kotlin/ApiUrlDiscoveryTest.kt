package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import okhttp3.OkHttp
import okhttp3.OkHttpClient
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

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
        assertEquals(
            "http://optional-https.wpmt.co",
            loginClient.loginUrl("http://optional-https.wpmt.co")
        )
    }

    @Test // Spec Example 6
    fun testHttpOnlySiteWithApplicationPasswordsEnabled() = runTest {
        assertEquals(
            "http://no-https.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("http://no-https.wpmt.co")
        )
    }

    @Test // Spec Example 7
    fun testAggressivelyCachedSiteWithNoLinkheader() = runTest {
        assertEquals(
            "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://aggressive-caching.wpmt.co")
        )
    }

    @Test // Spec Example 8
    fun testSiteWithApplicationPasswordsDisabledByWordFence() = runTest {
        assertEquals(
            "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://wordfence.wpmt.co")
        )
    }

    @Test // Spec Example 9
    fun testNotWordPressSite() = runTest {
        assertEquals(
            "https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://google.com")
        )
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

    @Test // Spec Example 13 (with invalid credentials)
    fun testWordPressHttpBasicWithInvalidCredentials() = runTest {
        assertEquals(
            "https://basic-auth.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://basic-auth.wpmt.co")
        )
    }

    @Test // Spec Example 13 (with valid credentials)
    fun testWordPressHttpBasicWithValidCredentials() = runTest {
        assertEquals(
            "https://basic-auth.wpmt.co/wordpress/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://basic-auth.wpmt.co")
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
        assertEquals(
            "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://aggressive-rate-limiting.wpmt.co")
        )
    }

    @Test // Spec Example 16
    fun testInvalidUrl() = runTest {
        assertEquals(
            "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://valid-looking-url-but-not-actually.foo")
        )
    }

    @Test // Spec Example 17
    fun testInvalidHTTPsFails() = runTest {
        assertEquals(
            "https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php",
            loginClient.loginUrl("https://wordpress-1315525-4803651.cloudwaysapps.com")
        )
    }

    @Test // Spec Example 17 (with exception)
    fun testInvalidHttpsWithExceptionWorks() = runTest {
        // TODO
    }

}
