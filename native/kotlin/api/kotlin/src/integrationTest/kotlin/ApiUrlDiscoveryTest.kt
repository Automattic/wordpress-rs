package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient()

    @Test
    fun testFindsCorrectApiUrls() = runTest {
        val apiDiscoveryResult =
            loginClient.apiDiscovery("https://automatticwidgets.wpcomstaging.com/")
        assertEquals(
            "https://automatticwidgets.wpcomstaging.com/wp-json/",
            apiDiscoveryResult.apiRootUrl.url()
        )
        assertEquals(
            "https://automatticwidgets.wpcomstaging.com/wp-admin/authorize-application.php",
            apiDiscoveryResult.apiDetails.findApplicationPasswordsAuthenticationUrl()
        )
    }
}
