package rs.wordpress.example.shared.ui.plugins

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.PluginListParams
import uniffi.wp_api.PluginWithEditContext
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import uniffi.wp_mobile.Account
import java.net.URI

class PluginListViewModel {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private var apiClient: WpApiClient? = null

    private val _plugins = MutableStateFlow<List<PluginWithEditContext>>(emptyList())
    val plugins: StateFlow<List<PluginWithEditContext>> = _plugins.asStateFlow()

    fun setAccount(account: Account) {
        apiClient = null
        _plugins.value = emptyList()
        when (account) {
            is Account.SelfHostedSite -> {
                apiClient = WpApiClient(
                    wpOrgSiteApiRootUrl = URI(account.siteApiRoot).toURL(),
                    authProvider = WpAuthenticationProvider.staticWithAuth(
                        wpAuthenticationFromUsernameAndPassword(account.username, account.password)
                    ),
                    interceptors = emptyList()
                )
                loadPlugins()
            }
            is Account.WpCom -> {
                // WP.com accounts not yet supported in this view model
            }
        }
    }

    private fun loadPlugins() {
        viewModelScope.launch(Dispatchers.IO) {
            apiClient?.let { client ->
                val pluginsResult = client.request { requestBuilder ->
                    requestBuilder.plugins().listWithEditContext(params = PluginListParams())
                }
                when (pluginsResult) {
                    is WpRequestResult.Success -> _plugins.value = pluginsResult.response.data
                    else -> _plugins.value = emptyList()
                }
            }
        }
    }
}
