package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.NavMenuItemListParams
import uniffi.wp_api.SparseNavMenuItemFieldWithEditContext
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertNotNull

class NavMenuItemsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val client = defaultApiClient()

    @Test
    fun testNavMenuItemListRequest() = runTest {
        val navMenuItemList = client.request { requestBuilder ->
            requestBuilder.navMenuItems().listWithEditContext(NavMenuItemListParams())
        }.assertSuccessAndRetrieveData().data
        assert(navMenuItemList.isNotEmpty())
    }

    @Test
    fun testRetrieveNavMenuItemRequest() = runTest {
        val navMenuItem = client.request { requestBuilder ->
            requestBuilder.navMenuItems()
                .retrieveWithEditContext(testCredentials.navMenuItemId)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(navMenuItem)
    }

    @Test
    fun testFilterNavMenuItemListRequest() = runTest {
        val navMenuItemList = client.request { requestBuilder ->
            requestBuilder.navMenuItems().filterListWithEditContext(
                NavMenuItemListParams(),
                listOf(
                    SparseNavMenuItemFieldWithEditContext.TITLE,
                    SparseNavMenuItemFieldWithEditContext.URL
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(navMenuItemList.isNotEmpty())
        assertNull(navMenuItemList.first().status)
    }

    @Test
    fun testFilterRetrieveNavMenuItemRequest() = runTest {
        val sparseNavMenuItem = client.request { requestBuilder ->
            requestBuilder.navMenuItems().filterRetrieveWithEditContext(
                testCredentials.navMenuItemId,
                listOf(
                    SparseNavMenuItemFieldWithEditContext.TITLE,
                    SparseNavMenuItemFieldWithEditContext.URL
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNotNull(sparseNavMenuItem)
        assertNull(sparseNavMenuItem.status)
    }

    @Test
    fun testErrorNavMenuItemListRequestInvalidPageNumber() = runTest {
        val params = NavMenuItemListParams(page = 99999999u)
        val result =
            client.request { requestBuilder ->
                requestBuilder.navMenuItems().listWithEditContext(params)
            }
        assert(result.wpErrorCode() is WpErrorCode.PostInvalidPageNumber)
    }
}
