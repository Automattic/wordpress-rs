package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SidebarUpdateParams
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertTrue

class SidebarsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val client = defaultApiClient()
    private val clientAsSubscriber = WpApiClient(
        testCredentials.apiRootUrl,
        WpAuthenticationProvider.staticWithUsernameAndPassword(
            username = testCredentials.subscriberUsername,
            password = testCredentials.subscriberPassword
        ),
        emptyList(),
        NetworkAvailabilityProvider { true }
    )

    @Test
    fun testSidebarsListRequest() = runTest {
        val sidebars = client.request { requestBuilder ->
            requestBuilder.sidebars().listWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assertTrue(sidebars.isNotEmpty())
    }

    @Test
    fun testSidebarsRetrieveRequest() = runTest {
        client.request { requestBuilder ->
            requestBuilder.sidebars()
                .retrieveWithEditContext("wp_inactive_widgets")
        }.assertSuccessAndRetrieveData()
    }

    @Test
    fun testSidebarsUpdateRequest() = runTest {
        val response = client.request { requestBuilder ->
            requestBuilder.sidebars()
                .update(
                    sidebarId = "wp_inactive_widgets",
                    params = SidebarUpdateParams(widgets = emptyList())
                )
        }.assertSuccessAndRetrieveData().data
        assertTrue(response.widgets.isEmpty())
    }

    @Test
    fun testSidebarsErrNotFound() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.sidebars()
                .retrieveWithViewContext("nonexistent_sidebar_that_does_not_exist")
        }
        assert(result.wpErrorCode() is WpErrorCode.SidebarNotFound)
    }

    @Test
    fun testSidebarsErrCannotManageWidgets() = runTest {
        val result = clientAsSubscriber.request { requestBuilder ->
            requestBuilder.sidebars().listWithEditContext()
        }
        assert(result.wpErrorCode() is WpErrorCode.CannotManageWidgets)
    }
}
