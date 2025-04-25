package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

class SiteSettingsEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testRetrieveSiteSettings() = runTest {
        val siteSettings = client.request { requestBuilder ->
            requestBuilder.siteSettings().retrieveWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assertEquals(FIRST_USER_EMAIL, siteSettings.email)
    }
}
