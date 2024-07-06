package rs.wordpress.example.ui.welcome

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import kotlinx.coroutines.runBlocking
import org.koin.android.ext.android.inject
import rs.wordpress.api.kotlin.WpLoginClient
import rs.wordpress.example.shared.App
import rs.wordpress.example.shared.repository.AuthenticationRepository

class WelcomeActivity : ComponentActivity() {
    private val authRepository: AuthenticationRepository by inject()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            App(authenticationEnabled = true, ::authenticateSite)
        }
    }

    private fun authenticateSite(url: String) {
        val authenticationUrl = runBlocking {
            WpLoginClient().apiDiscovery(url)
                .getOrThrow().apiDetails.findApplicationPasswordsAuthenticationUrl()
        }
        val uriBuilder = Uri.parse(authenticationUrl).buildUpon()

        uriBuilder
            .appendQueryParameter("app_name", "WordPressRsAndroidExample")
            .appendQueryParameter("app_id", "00000000-0000-4000-8000-000000000000")
            .appendQueryParameter("success_url", "wordpressrsexample://authorized")

        uriBuilder.build().let { uri ->
            val i = Intent(Intent.ACTION_VIEW, uri)
            startActivity(i)
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)

        intent.data?.let {
            val siteUrl = it.getQueryParameter("site_url")
            val username = it.getQueryParameter("user_login")
            val password = it.getQueryParameter("password")

            if (siteUrl != null && username != null && password != null) {
                authRepository.addAuthenticatedSite(siteUrl, username, password)
                onBackPressedDispatcher.onBackPressed()
            }
        }
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    App(authenticationEnabled = false) {}
}
