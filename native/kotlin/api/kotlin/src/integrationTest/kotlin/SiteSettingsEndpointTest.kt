package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals

class SiteSettingsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val client = WpApiClient(
        siteUrl, wpAuthenticationFromUsernameAndPassword(
            username = testCredentials.adminUsername, password = testCredentials.adminPassword
        )
    )

    @Test
    fun testRetrieveSiteSettings() = runTest {
        val siteSettings = client.request { requestBuilder ->
            requestBuilder.siteSettings().retrieveWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assertEquals(FIRST_USER_EMAIL, siteSettings.email)
    }
}