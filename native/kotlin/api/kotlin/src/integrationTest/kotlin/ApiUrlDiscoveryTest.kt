package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

class ApiUrlDiscoveryTest {
    private val loginClient: WpLoginClient = WpLoginClient()

    @Test
    fun testFindsCorrectApiUrls() = runTest {
        val apiUrls = loginClient.apiDiscovery("https://orchestremetropolitain.com/fr/").getOrThrow()
        assertEquals("https://orchestremetropolitain.com/wp-json/", apiUrls.apiRootUrl)
        assertEquals(
            "https://orchestremetropolitain.com/wp-admin/authorize-application.php",
            apiUrls.apiDetails.findApplicationPasswordsAuthenticationUrl()
        )
    }
}
