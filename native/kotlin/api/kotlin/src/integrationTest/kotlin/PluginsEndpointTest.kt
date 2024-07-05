package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PluginCreateParams
import uniffi.wp_api.PluginListParams
import uniffi.wp_api.PluginSlug
import uniffi.wp_api.PluginStatus
import uniffi.wp_api.PluginWpOrgDirectorySlug
import uniffi.wp_api.SparsePluginField
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpRestErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class PluginsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
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
        val result = client.request { requestBuilder ->
            requestBuilder.plugins().listWithEditContext(params = PluginListParams())
        }
        assert(result is WpRequestSuccess)
        val pluginList = (result as WpRequestSuccess).data
        assertEquals(NUMBER_OF_PLUGINS, pluginList.count())
    }

    @Test
    fun testFilterPluginListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.plugins().filterList(
                context = WpContext.EDIT,
                params = PluginListParams(),
                fields = listOf(SparsePluginField.AUTHOR, SparsePluginField.VERSION)
            )
        }
        assert(result is WpRequestSuccess)
        val pluginList = (result as WpRequestSuccess).data
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
        val result = client.request { requestBuilder ->
            requestBuilder.plugins().filterRetrieve(
                context = WpContext.EDIT,
                pluginSlug = pluginSlug,
                fields = listOf(
                    SparsePluginField.PLUGIN,
                    SparsePluginField.REQUIRES_WP,
                    SparsePluginField.STATUS
                )
            )
        }
        assert(result is WpRequestSuccess)
        val sparsePlugin = (result as WpRequestSuccess).data
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
        assert(result is RecognizedRestError)
        assert((result as RecognizedRestError).error.code is WpRestErrorCode.CannotInstallPlugin)
    }

    @Test
    fun testDeletePluginErrCannotDeleteActivePlugin() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.plugins().delete(PluginSlug(HELLO_DOLLY_PLUGIN_SLUG))
        }
        assert(result is RecognizedRestError)
        assert((result as RecognizedRestError).error.code is WpRestErrorCode.CannotDeleteActivePlugin)
    }
}
