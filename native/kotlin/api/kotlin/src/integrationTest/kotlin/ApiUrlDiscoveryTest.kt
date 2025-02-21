package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.WpLoginClientConfiguration
import kotlin.test.assertEquals

class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient(config = WpLoginClientConfiguration(true,
        1u, 3u))

    @Test
    fun testFindsCorrectApiUrls() = runTest {
        val apiDiscoveryResult =
            loginClient.apiDiscovery("https://automatticwidgets.wpcomstaging.com/")
        assert(apiDiscoveryResult.isSuccessful)
        assertEquals(
            "https://automatticwidgets.wpcomstaging.com/wp-json/",
            apiDiscoveryResult.successfulAttempt?.apiRootUrl()?.url()
        )
        assertEquals(
            "https://automatticwidgets.wpcomstaging.com/wp-admin/authorize-application.php",
            apiDiscoveryResult.successfulAttempt?.apiDetails()
                ?.findApplicationPasswordsAuthenticationUrl()
        )
    }
}
