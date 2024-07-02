package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient()

    @Test
    fun testFindsCorrectApiUrls() = runTest {
        val urlDiscovery = loginClient.apiDiscovery("https://orchestremetropolitain.com/fr/").getOrThrow()
        assertEquals("https://orchestremetropolitain.com/wp-json/", urlDiscovery.apiRootUrl.url())
        assertEquals(
            "https://orchestremetropolitain.com/wp-admin/authorize-application.php",
            urlDiscovery.apiDetails.findApplicationPasswordsAuthenticationUrl()
        )
    }
}
