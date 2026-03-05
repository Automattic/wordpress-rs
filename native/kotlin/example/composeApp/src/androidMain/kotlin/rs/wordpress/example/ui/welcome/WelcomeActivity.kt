package rs.wordpress.example.ui.welcome

import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import kotlinx.coroutines.runBlocking
import org.koin.android.ext.android.inject
import rs.wordpress.api.kotlin.ApiDiscoveryResult
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpComApiClient
import rs.wordpress.api.kotlin.WpLoginClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.example.WpComCredentials
import rs.wordpress.example.shared.App
import rs.wordpress.example.shared.repository.AuthenticationRepository
import androidx.core.net.toUri
import rs.wordpress.api.kotlin.toURL
import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.DiscoveredAuthenticationMechanism
import uniffi.wp_api.OAuth2Configuration
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpComSiteIdentifier
import uniffi.wp_api.wordpressComOauth2Configuration
import uniffi.wp_api.WpComOauthScope
import java.util.UUID

private const val TAG = "WelcomeActivity"
private const val OAUTH2_REDIRECT_URI = "wordpressrsexample://oauth2-callback"

class WelcomeActivity : ComponentActivity() {
    private val authRepository: AuthenticationRepository by inject()
    private val networkAvailabilityProvider: NetworkAvailabilityProvider by inject()
    private var apiDiscoverySuccess: AutoDiscoveryAttemptSuccess? = null
    private var oauthConfiguration: OAuth2Configuration? = null
    private var oauthState: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            App(authenticationEnabled = true, ::authenticateSite)
        }
    }

    private fun authenticateSite(url: String) {
        val apiDiscoveryResult = runBlocking {
            WpLoginClient(emptyList(), networkAvailabilityProvider).apiDiscovery(url)
        }

        apiDiscoveryResult.userFacingErrorMessage(url)?.let { errorMessage ->
            Log.e(TAG, "API discovery failed for $url: $apiDiscoveryResult")
            Toast.makeText(this, errorMessage, Toast.LENGTH_LONG).show()
            return
        }

        val success = (apiDiscoveryResult as ApiDiscoveryResult.Success).success
        apiDiscoverySuccess = success

        when (val auth = success.authentication) {
            is DiscoveredAuthenticationMechanism.ApplicationPasswords -> {
                startApplicationPasswordsFlow(auth)
            }
            is DiscoveredAuthenticationMechanism.OAuth2 -> {
                startOAuth2Flow(success, auth)
            }
        }
    }

    private fun startApplicationPasswordsFlow(auth: DiscoveredAuthenticationMechanism.ApplicationPasswords) {
        val uriBuilder = auth.authenticationUrl.url().toUri().buildUpon()

        uriBuilder
            .appendQueryParameter("app_name", "WordPressRsAndroidExample")
            .appendQueryParameter("app_id", "00000000-0000-4000-8000-000000000000")
            .appendQueryParameter("success_url", "wordpressrsexample://authorized")

        uriBuilder.build().let { uri ->
            val i = Intent(Intent.ACTION_VIEW, uri)
            startActivity(i)
        }
    }

    private fun startOAuth2Flow(
        success: AutoDiscoveryAttemptSuccess,
        auth: DiscoveredAuthenticationMechanism.OAuth2
    ) {
        val clientId = WpComCredentials.CLIENT_ID
        val clientSecret = WpComCredentials.CLIENT_SECRET

        if (clientId == null || clientSecret == null) {
            Log.e(TAG, "WP.com OAuth credentials not configured. Add wp_com_test_credentials.json to the repository root.")
            Toast.makeText(this, "WP.com OAuth credentials not configured", Toast.LENGTH_LONG).show()
            return
        }

        val configuration = wordpressComOauth2Configuration(
            clientId = clientId.toULong(),
            clientSecret = clientSecret,
            redirectUri = OAUTH2_REDIRECT_URI,
            scope = listOf(WpComOauthScope.GLOBAL)
        )
        oauthConfiguration = configuration

        val state = UUID.randomUUID().toString()
        oauthState = state

        val host = java.net.URI(success.parsedSiteUrl.url()).host
        val blogId = if (host != null) WpComSiteIdentifier.Slug(host) else null

        val authUrl = configuration.buildTokenRequestUrl(state, blogId)
        val i = Intent(Intent.ACTION_VIEW, authUrl.url().toUri())
        startActivity(i)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)

        val data = intent.data ?: return

        when (data.host) {
            "authorized" -> handleApplicationPasswordsCallback(data)
            "oauth2-callback" -> handleOAuth2Callback(data)
        }
    }

    private fun handleApplicationPasswordsCallback(data: android.net.Uri) {
        val siteUrl = data.getQueryParameter("site_url")
        val username = data.getQueryParameter("user_login")
        val password = data.getQueryParameter("password")

        if (siteUrl != null && username != null && password != null) {
            val discoverySuccess = apiDiscoverySuccess ?: run {
                Log.e(TAG, "API discovery state lost (activity may have been recreated)")
                Toast.makeText(this, "Login session expired. Please try again.", Toast.LENGTH_LONG).show()
                return
            }
            authRepository.addAuthenticatedSite(
                discoverySuccess.parsedSiteUrl.toURL(),
                discoverySuccess.apiRootUrl.toURL(),
                username,
                password
            )
            onBackPressedDispatcher.onBackPressed()
        }
    }

    private fun handleOAuth2Callback(data: android.net.Uri) {
        val configuration = oauthConfiguration ?: run {
            Log.e(TAG, "OAuth2 callback received but no configuration is set")
            Toast.makeText(this, "Login session expired. Please try again.", Toast.LENGTH_LONG).show()
            return
        }

        val tokenResponse = try {
            configuration.parseTokenResponse(
                url = data.toString(),
                expectedState = oauthState
            )
        } catch (e: Exception) {
            Log.e(TAG, "Failed to parse OAuth2 callback", e)
            Toast.makeText(this, "OAuth2 login failed: ${e.message}", Toast.LENGTH_LONG).show()
            return
        }

        val requestParams = configuration.buildTokenRequestParameters(tokenResponse.code)

        val response = runBlocking {
            val client = WpComApiClient(
                authProvider = WpAuthenticationProvider.none(),
                interceptors = emptyList(),
                networkAvailabilityProvider = networkAvailabilityProvider
            )
            client.request { it.oauth2().requestToken(requestParams) }
        }

        when (response) {
            is WpRequestResult.Success -> {
                val tokenData = response.response.data
                val blogId = tokenData.blogId
                val siteApiRoot = if (blogId != null) {
                    "https://public-api.wordpress.com/wp/v2/sites/$blogId"
                } else {
                    "https://public-api.wordpress.com/wp/v2"
                }
                authRepository.addWpComAccount(
                    token = tokenData.accessToken,
                    siteApiRoot = siteApiRoot,
                    displayName = tokenData.blogUrl ?: "WordPress.com"
                )
                onBackPressedDispatcher.onBackPressed()
            }
            else -> {
                Log.e(TAG, "Token exchange failed: $response")
                Toast.makeText(this, "Token exchange failed", Toast.LENGTH_LONG).show()
            }
        }
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    App(authenticationEnabled = false) {}
}
