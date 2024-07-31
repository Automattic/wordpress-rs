package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient()

    @Test
    fun testFindsCorrectApiUrls() = runTest {
        val urlDiscovery = loginClient.apiDiscovery("https://automatticwidgets.wpcomstaging.com/").getOrThrow()
        assertEquals("https://automatticwidgets.wpcomstaging.com/wp-json/", urlDiscovery.apiRootUrl.url())
        assertEquals(
            "https://automatticwidgets.wpcomstaging.com/wp-admin/authorize-application.php",
            urlDiscovery.apiDetails.findApplicationPasswordsAuthenticationUrl()
        )
    }
}
