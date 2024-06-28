package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.UrlDiscoveryResult
import kotlin.test.assertEquals

class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient()

    @Test
    fun testFindsCorrectApiUrls() = runTest {
        val urlDiscoveryResult = loginClient.apiDiscovery("https://orchestremetropolitain.com/fr/")
        assert(urlDiscoveryResult is UrlDiscoveryResult.Success)
        val urlDiscoverySuccess = urlDiscoveryResult as UrlDiscoveryResult.Success;
        assertEquals("https://orchestremetropolitain.com/wp-json/", urlDiscoverySuccess.apiRootUrl.url())
        assertEquals(
            "https://orchestremetropolitain.com/wp-admin/authorize-application.php",
            urlDiscoverySuccess.apiDetails.findApplicationPasswordsAuthenticationUrl()
        )
    }
}
