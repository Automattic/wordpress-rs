package rs.wordpress.example.shared.ui.welcome

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.AccountRepository

class WelcomeViewModel(private val accountRepository: AccountRepository): ViewModel() {
    private val _sites = MutableStateFlow<List<Account>>(emptyList())
    val sites: StateFlow<List<Account>> = _sites.asStateFlow()

    init {
        refreshSites()
    }

    fun refreshSites() {
        viewModelScope.launch(Dispatchers.IO) {
            _sites.value = accountRepository.all()
        }
    }

    fun deleteSite(account: Account) {
        viewModelScope.launch(Dispatchers.IO) {
            accountRepository.remove(account.id())
            _sites.value = accountRepository.all()
        }
    }
}
