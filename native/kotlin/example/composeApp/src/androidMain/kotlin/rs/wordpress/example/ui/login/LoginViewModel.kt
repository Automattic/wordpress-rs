package rs.wordpress.example.ui.login

import android.net.Uri
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

    fun authenticationUrl(inputSiteUrl: String): Uri? {
        val uriBuilder = Uri.parse(inputSiteUrl).buildUpon()

        uriBuilder.appendPath("wp-admin")
            .appendPath("authorize-application.php")
            .appendQueryParameter("app_name", "WordPressRsAndroidExample")
            .appendQueryParameter("app_id", "00000000-0000-4000-8000-000000000000")
            .appendQueryParameter("success_url", "wordpressrsexample://authorized")
        return uriBuilder.build()
    }
}