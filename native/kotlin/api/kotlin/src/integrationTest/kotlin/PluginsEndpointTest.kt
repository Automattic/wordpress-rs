package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PluginCreateParams
import uniffi.wp_api.PluginListParams
import uniffi.wp_api.PluginSlug
import uniffi.wp_api.PluginStatus
import uniffi.wp_api.PluginWpOrgDirectorySlug
import uniffi.wp_api.SparsePluginFieldWithEditContext
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class PluginsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val client = WpApiClient(
        siteUrl, wpAuthenticationFromUsernameAndPassword(
            username = testCredentials.adminUsername, password = testCredentials.adminPassword
        )
    )
    private val clientAsSubscriber = WpApiClient(
        siteUrl, wpAuthenticationFromUsernameAndPassword(
            username = testCredentials.subscriberUsername,
            password = testCredentials.subscriberPassword
        )
    )

    @Test
    fun testPluginListRequest() = runTest {
        val pluginList = client.request { requestBuilder ->
            requestBuilder.plugins().listWithEditContext(params = PluginListParams())
        }.assertSuccessAndRetrieveData().data
        assertEquals(NUMBER_OF_PLUGINS, pluginList.count())
    }

    @Test
    fun testFilterPluginListRequest() = runTest {
        val pluginList = client.request { requestBuilder ->
            requestBuilder.plugins().filterListWithEditContext(
                params = PluginListParams(),
                fields = listOf(
                    SparsePluginFieldWithEditContext.AUTHOR,
                    SparsePluginFieldWithEditContext.VERSION
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(NUMBER_OF_PLUGINS, pluginList.count())
        pluginList.forEach {
            assertNotNull(it.author)
            assertNotNull(it.version)
            assertNull(it.plugin)
            assertNull(it.pluginUri)
        }
    }

    @Test
    fun testFilterRetrievePluginRequest() = runTest {
        val pluginSlug = PluginSlug(HELLO_DOLLY_PLUGIN_SLUG)
        val sparsePlugin = client.request { requestBuilder ->
            requestBuilder.plugins().filterRetrieveWithEditContext(
                pluginSlug = pluginSlug,
                fields = listOf(
                    SparsePluginFieldWithEditContext.PLUGIN,
                    SparsePluginFieldWithEditContext.REQUIRES_WP,
                    SparsePluginFieldWithEditContext.STATUS
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(pluginSlug, sparsePlugin.plugin)
        assertNotNull(sparsePlugin.requiresWp)
        assertNotNull(sparsePlugin.status)
        assertNull(sparsePlugin.pluginUri)
        assertNull(sparsePlugin.description)
    }

    @Test
    fun testCreatePluginErrCannotInstallPlugin() = runTest {
        val result = clientAsSubscriber.request { requestBuilder ->
            requestBuilder.plugins().create(
                params = PluginCreateParams(
                    slug = PluginWpOrgDirectorySlug(WP_ORG_PLUGIN_SLUG_CLASSIC_WIDGETS),
                    status = PluginStatus.ACTIVE
                )
            )
        }
        assert(result.wpErrorCode() is WpErrorCode.CannotInstallPlugin)
    }

    @Test
    fun testDeletePluginErrCannotDeleteActivePlugin() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.plugins().delete(PluginSlug(HELLO_DOLLY_PLUGIN_SLUG))
        }
        assert(result.wpErrorCode() is WpErrorCode.CannotDeleteActivePlugin)
    }
}
