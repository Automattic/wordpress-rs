package rs.wordpress.example.shared.ui.sitehealth

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.Dispatchers
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.shared.ui.components.errorDescription
import uniffi.wp_api.WpSiteHealthTest

class SiteHealthViewModel(private val apiClient: WpApiClient) : ViewModel() {

    private val _tests = MutableStateFlow<List<WpSiteHealthTest>>(emptyList())
    val tests: StateFlow<List<WpSiteHealthTest>> = _tests.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    init {
        loadTests()
    }

    private fun loadTests() {
        viewModelScope.launch(Dispatchers.IO) {
            var lastError: String? = null

            val backgroundUpdates = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().backgroundUpdates() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> { lastError = r.errorDescription(); null }
                }
            }
            val loopbackRequests = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().loopbackRequests() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> { lastError = r.errorDescription(); null }
                }
            }
            val httpsStatus = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().httpsStatus() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> { lastError = r.errorDescription(); null }
                }
            }
            val dotorgCommunication = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().dotorgCommunication() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> { lastError = r.errorDescription(); null }
                }
            }
            val authorizationHeader = async {
                when (val r = apiClient.request { it.wpSiteHealthTests().authorizationHeader() }) {
                    is WpRequestResult.Success -> r.response.data
                    else -> { lastError = r.errorDescription(); null }
                }
            }

            val results = listOfNotNull(
                backgroundUpdates.await(),
                loopbackRequests.await(),
                httpsStatus.await(),
                dotorgCommunication.await(),
                authorizationHeader.await()
            )
            _tests.value = results
            if (results.isEmpty()) {
                _error.value = lastError
            }
            _isLoading.value = false
        }
    }
}
