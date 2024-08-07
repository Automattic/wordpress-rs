package rs.wordpress.example.shared.ui.plugins

import kotlinx.coroutines.runBlocking
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.shared.domain.AuthenticatedSite
import rs.wordpress.example.shared.repository.AuthenticationRepository
import uniffi.wp_api.PluginListParams
import uniffi.wp_api.PluginWithEditContext

class PluginListViewModel(private val authRepository: AuthenticationRepository) {
    private var apiClient: WpApiClient? = null

    fun setAuthenticatedSite(authenticatedSite: AuthenticatedSite) {
        apiClient = null
        authRepository.authenticationForSite(authenticatedSite)?.let {
            apiClient = WpApiClient(siteUrl = authenticatedSite.url, authentication = it)
        }
    }

    fun fetchPlugins(): List<PluginWithEditContext> {
        apiClient?.let { apiClient ->
            val pluginsResult = runBlocking {
                apiClient.request { requestBuilder ->
                    requestBuilder.plugins().listWithEditContext(params = PluginListParams())
                }
            }
            return when (pluginsResult) {
                is WpRequestResult.WpRequestSuccess -> pluginsResult.data
                else -> listOf()
            }
        }
        return listOf()
    }
}