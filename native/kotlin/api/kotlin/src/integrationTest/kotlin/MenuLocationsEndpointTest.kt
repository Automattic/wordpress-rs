package rs.wordpress.api.kotlin

import kotlin.test.assertEquals

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
class MenuLocationsEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testMenuLocationsListRequest() = runTest {
        val menuLocations = client.request { requestBuilder ->
            requestBuilder.menuLocations().listWithEditContext()
        }.assertSuccessAndRetrieveData().data.locations

        assertNotNull(menuLocations)
        assertTrue(menuLocations.isNotEmpty())
    }

    @Test
    fun testMenuLocationsRetrieveRequest() = runTest {
        client.request { requestBuilder ->
            requestBuilder.menuLocations()
                .retrieveWithEditContext(TestCredentials.INSTANCE.primaryMenuLocation)
        }.assertSuccessAndRetrieveData()
    }

    @Test
    fun testMenuLocationsErrMenuLocationInvalid() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.menuLocations()
                .retrieveWithViewContext("invalid_location_that_does_not_exist")
        }
        assertEquals(WpErrorCode.MENU_LOCATION_INVALID, result.wpErrorCode())
    }
}
