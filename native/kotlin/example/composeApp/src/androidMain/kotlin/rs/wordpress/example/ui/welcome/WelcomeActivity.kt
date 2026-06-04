package rs.wordpress.example.ui.welcome

import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import androidx.core.net.toUri
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import org.koin.android.ext.android.inject
import rs.wordpress.api.kotlin.ApiDiscoveryResult
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.api.kotlin.WpComApiClient
import rs.wordpress.api.kotlin.WpLoginClient
import rs.wordpress.api.kotlin.WpRequestResult
import rs.wordpress.api.kotlin.toURL
import rs.wordpress.example.WpComCredentials
import rs.wordpress.example.shared.App
import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.DiscoveredAuthenticationMechanism
import uniffi.wp_api.OAuth2Configuration
import uniffi.wp_api.TokenRequestParameters
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpComOauthScope
import uniffi.wp_api.WpComSiteIdentifier
import uniffi.wp_api.applicationPasswordsUrl
import uniffi.wp_api.buildTokenRequestUrl
import uniffi.wp_api.parseAuthorizationUrl
import uniffi.wp_api.wordpressComOauth2Configuration
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.AccountRepository

class WelcomeActivity : ComponentActivity() {
    private val accountRepository: AccountRepository by inject()
    private val networkAvailabilityProvider: NetworkAvailabilityProvider by inject()
    private var apiDiscoverySuccess: AutoDiscoveryAttemptSuccess? = null
    private var wpComOAuthState: String? = null
    private var siteSpecificOAuthConfig: OAuth2Configuration? = null
    private var siteSpecificOAuthState: String? = null
    private var discoveredSiteHost: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val wpComAuth = if (WpComCredentials.CLIENT_ID != null && WpComCredentials.CLIENT_SECRET != null) {
            ::authenticateWpCom
        } else {
            null
        }

        setContent {
            App(authenticationEnabled = true, ::authenticateSite, wpComAuth)
        }
    }

    private fun authenticateSite(url: String, onSuccess: () -> Unit, onError: (String) -> Unit) {
        lifecycleScope.launch {
            try {
                val apiDiscoveryResult = WpLoginClient(emptyList(), networkAvailabilityProvider).apiDiscovery(url)
                val success = when (apiDiscoveryResult) {
                    is ApiDiscoveryResult.Success -> apiDiscoveryResult.success
                    else -> {
                        runOnUiThread {
                            val message = apiDiscoveryResult.userFacingErrorMessage(url)
                                ?: "Failed to discover site API"
                            Toast.makeText(this@WelcomeActivity, message, Toast.LENGTH_LONG).show()
                            onError(message)
                        }
                        return@launch
                    }
                }

                apiDiscoverySuccess = success

                when (success.authentication) {
                    is DiscoveredAuthenticationMechanism.ApplicationPasswords -> {
                        val authUrl = applicationPasswordsUrl(success.authentication)
                        if (authUrl == null) {
                            runOnUiThread {
                                val message = "Application passwords not supported"
                                Toast.makeText(this@WelcomeActivity, message, Toast.LENGTH_LONG).show()
                                onError(message)
                            }
                            return@launch
                        }
                        val uriBuilder = authUrl.url().toUri().buildUpon()

                        uriBuilder
                            .appendQueryParameter("app_name", "WordPressRsAndroidExample")
                            .appendQueryParameter("app_id", "00000000-0000-4000-8000-000000000000")
                            .appendQueryParameter("success_url", SELF_HOSTED_REDIRECT_URI)

                        runOnUiThread { onSuccess() }
                        uriBuilder.build().let { uri ->
                            startActivity(Intent(Intent.ACTION_VIEW, uri))
                        }
                    }
                    is DiscoveredAuthenticationMechanism.OAuth2 -> {
                        val clientId = WpComCredentials.CLIENT_ID ?: return@launch
                        val clientSecret = WpComCredentials.CLIENT_SECRET ?: return@launch

                        val config = wordpressComOauth2Configuration(
                            clientId = clientId,
                            clientSecret = clientSecret,
                            redirectUri = WPCOM_REDIRECT_URI,
                            scope = listOf(WpComOauthScope.GLOBAL)
                        )
                        siteSpecificOAuthConfig = config

                        val host = success.parsedSiteUrl.toURL().toURI().host
                        discoveredSiteHost = host
                        val state = java.util.UUID.randomUUID().toString()
                        siteSpecificOAuthState = state

                        val authUrl = config.buildTokenRequestUrl(
                            state = state,
                            blog = WpComSiteIdentifier.Slug(value = host)
                        )
                        runOnUiThread { onSuccess() }
                        startActivity(Intent(Intent.ACTION_VIEW, authUrl.url().toUri()))
                    }
                }
            } catch (e: Exception) {
                runOnUiThread {
                    val message = e.localizedMessage ?: "An unexpected error occurred"
                    Toast.makeText(this@WelcomeActivity, message, Toast.LENGTH_LONG).show()
                    onError(message)
                }
            }
        }
    }

    private fun authenticateWpCom() {
        val clientId = WpComCredentials.CLIENT_ID!!
        val state = java.util.UUID.randomUUID().toString()
        wpComOAuthState = state

        val url = buildTokenRequestUrl(
            clientId = clientId,
            redirectUri = WPCOM_REDIRECT_URI,
            scope = listOf(WpComOauthScope.GLOBAL),
            state = state,
            blog = null
        )

        val intent = Intent(Intent.ACTION_VIEW, url.url().toUri())
        startActivity(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)

        intent.data?.let { uri ->
            when (uri.host) {
                "authorized" -> handleSelfHostedCallback(uri)
                "wpcom-authorized" -> handleWpComCallback(uri)
            }
        }
    }

    private fun handleSelfHostedCallback(uri: android.net.Uri) {
        val siteUrl = uri.getQueryParameter("site_url")
        val username = uri.getQueryParameter("user_login")
        val password = uri.getQueryParameter("password")

        if (siteUrl != null && username != null && password != null) {
            val discoverySuccess = apiDiscoverySuccess
                ?: throw IllegalStateException("Api discovery has to be successful before authentication")
            accountRepository.store(
                Account.SelfHostedSite(
                    id = 0uL,
                    domain = discoverySuccess.parsedSiteUrl.toURL().toString(),
                    username = username,
                    password = password,
                    siteApiRoot = discoverySuccess.apiRootUrl.toURL().toString()
                )
            )
            onBackPressedDispatcher.onBackPressed()
        }
    }

    private fun handleWpComCallback(uri: android.net.Uri) {
        lifecycleScope.launch {
            try {
                val isSiteSpecific = siteSpecificOAuthState != null
                val config = siteSpecificOAuthConfig

                if (isSiteSpecific && config != null) {
                    // Site-specific WP.com OAuth flow (discovered via self-hosted URL)
                    val result = config.parseTokenResponse(
                        url = uri.toString(),
                        expectedState = siteSpecificOAuthState
                    )
                    val tokenParams = config.buildTokenRequestParameters(code = result.code)

                    val wpComClient = WpComApiClient(
                        authProvider = WpAuthenticationProvider.none(),
                        interceptors = emptyList(),
                        networkAvailabilityProvider = networkAvailabilityProvider
                    )

                    val tokenResult = wpComClient.request { client ->
                        client.oauth2().requestToken(tokenParams)
                    }

                    when (tokenResult) {
                        is WpRequestResult.Success -> {
                            val tokenResponse = tokenResult.response.data
                            val blogId = tokenResponse.blogId
                                ?: throw IllegalStateException("Expected blog_id in site-specific token response")
                            accountRepository.store(
                                Account.WpCom(
                                    id = 0uL,
                                    username = discoveredSiteHost
                                        ?: tokenResponse.blogUrl
                                        ?: "WordPress.com",
                                    token = tokenResponse.accessToken,
                                    siteApiRoot = blogId.toString()
                                )
                            )
                            siteSpecificOAuthState = null
                            siteSpecificOAuthConfig = null
                            discoveredSiteHost = null
                            onBackPressedDispatcher.onBackPressed()
                        }
                        else -> {
                            // Token exchange failed – stay on login screen
                        }
                    }
                } else {
                    // Global WP.com OAuth flow
                    val result = parseAuthorizationUrl(uri.toString())
                    if (result.state != wpComOAuthState) return@launch

                    val clientId = WpComCredentials.CLIENT_ID ?: return@launch
                    val clientSecret = WpComCredentials.CLIENT_SECRET ?: return@launch

                    val wpComClient = WpComApiClient(
                        authProvider = WpAuthenticationProvider.none(),
                        interceptors = emptyList(),
                        networkAvailabilityProvider = networkAvailabilityProvider
                    )

                    val tokenResult = wpComClient.request { client ->
                        client.oauth2().requestToken(
                            TokenRequestParameters(
                                clientId = clientId,
                                clientSecret = clientSecret,
                                code = result.code,
                                redirectUri = WPCOM_REDIRECT_URI
                            )
                        )
                    }

                    when (tokenResult) {
                        is WpRequestResult.Success -> {
                            val tokenResponse = tokenResult.response.data
                            accountRepository.store(
                                Account.WpCom(
                                    id = 0uL,
                                    username = tokenResponse.blogUrl ?: "WordPress.com",
                                    token = tokenResponse.accessToken,
                                    siteApiRoot = ""
                                )
                            )
                            onBackPressedDispatcher.onBackPressed()
                        }
                        else -> {
                            // Token exchange failed – stay on login screen
                        }
                    }
                }
            } catch (_: Exception) {
                // Authorization URL parsing failed – stay on login screen
            }
        }
    }

    companion object {
        private const val SELF_HOSTED_REDIRECT_URI = "wordpressrsexample://authorized"
        private const val WPCOM_REDIRECT_URI = "wordpressrsexample://wpcom-authorized"
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    App(authenticationEnabled = false, authenticateSite = { _, _, _ -> }, authenticateWpCom = null)
}
