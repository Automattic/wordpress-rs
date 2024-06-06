package rs.wordpress.example.ui.login

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch
import rs.wordpress.example.shared.repository.AuthenticationRepository

class LoginViewModel(private val authRepository: AuthenticationRepository): ViewModel() {
    fun addAuthenticatedSite(siteUrl: String, username: String, password: String) {
        viewModelScope.launch {
            val user = authRepository.addAuthenticatedSite(siteUrl, username, password)
            Log.e("WORDPRESS_RS_EXAMPLE", "Authenticated successfully. Here is the user: $user")
        }
    }
}