package rs.wordpress.example

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import org.koin.android.ext.android.inject
import rs.wordpress.example.ui.login.LoginScreen
import rs.wordpress.example.ui.login.LoginViewModel

class MainActivity : ComponentActivity() {
    private val loginViewModel: LoginViewModel by inject()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            LoginScreen(::authenticateSite)
        }
    }

    private fun authenticateSite(url: String) {
        val uriBuilder = Uri.parse(url).buildUpon()

        uriBuilder.appendPath("wp-admin")
            .appendPath("authorize-application.php")
            .appendQueryParameter("app_name", "WordPressRsAndroidExample")
            .appendQueryParameter("app_id", "00000000-0000-4000-8000-000000000000")
            .appendQueryParameter("success_url", "wordpressrsexample://authorized")
        openUrl(uriBuilder.build())
    }

    private fun openUrl(uri: Uri) {
        val i = Intent(Intent.ACTION_VIEW, uri)
        startActivity(i)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)

        intent.data?.let {
            val siteUrl = it.getQueryParameter("site_url")
            val userLogin = it.getQueryParameter("user_login")
            val password = it.getQueryParameter("password")

            if (siteUrl != null && userLogin != null && password != null) {
                loginViewModel.addAuthenticatedSite(siteUrl, userLogin, password)
            } else {
                // TODO
            }
        }
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    LoginScreen {}
}
