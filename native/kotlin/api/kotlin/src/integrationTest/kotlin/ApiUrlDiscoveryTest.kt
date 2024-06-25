package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.findApiUrls
import kotlin.test.assertEquals

class ApiUrlDiscoveryTest {
    private val requestExecutor: RequestExecutor = WpRequestExecutor()

    @Test
    fun testFindsCorrectApiUrls() = runTest {
        val apiUrls = findApiUrls("https://orchestremetropolitain.com/fr/", requestExecutor)
        assertEquals("https://orchestremetropolitain.com/wp-json/", apiUrls.apiRootUrl)
        assertEquals(
            "https://orchestremetropolitain.com/wp-admin/authorize-application.php",
            apiUrls.apiDetails.findApplicationPasswordsAuthenticationUrl()
        )
    }
}