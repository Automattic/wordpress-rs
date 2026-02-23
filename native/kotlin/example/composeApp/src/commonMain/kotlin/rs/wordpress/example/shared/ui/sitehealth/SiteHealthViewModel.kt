package rs.wordpress.example.shared.ui.sitehealth

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.WpSiteHealthTest

class SiteHealthViewModel(private val apiClient: WpApiClient) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _tests = MutableStateFlow<List<WpSiteHealthTest>>(emptyList())
    val tests: StateFlow<List<WpSiteHealthTest>> = _tests.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadTests()
    }

    private fun loadTests() {
        viewModelScope.launch(Dispatchers.IO) {
            val backgroundUpdates = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().backgroundUpdates() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> null
                }
            }
            val loopbackRequests = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().loopbackRequests() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> null
                }
            }
            val httpsStatus = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().httpsStatus() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> null
                }
            }
            val dotorgCommunication = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().dotorgCommunication() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> null
                }
            }
            val authorizationHeader = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().authorizationHeader() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> null
                }
            }

            _tests.value = listOfNotNull(
                backgroundUpdates.await(),
                loopbackRequests.await(),
                httpsStatus.await(),
                dotorgCommunication.await(),
                authorizationHeader.await()
            )
            _isLoading.value = false
        }
    }
}
