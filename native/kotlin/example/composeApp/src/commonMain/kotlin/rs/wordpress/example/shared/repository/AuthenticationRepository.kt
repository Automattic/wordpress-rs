package rs.wordpress.example.shared.repository

import rs.wordpress.example.shared.domain.AuthenticatedSite
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import java.net.URI
import java.net.URL

class AuthenticationRepository(
    localTestSiteUrl: String,
    localTestSiteUsername: String,
    localTestSitePassword: String
) {
    private val authenticatedSites = mutableMapOf<AuthenticatedSite, WpAuthentication>()

    init {
        addAuthenticatedSite(
            URI(localTestSiteUrl).toURL(),
            URI("$localTestSiteUrl/wp-json").toURL(),
            localTestSiteUsername,
            localTestSitePassword
        )
    }

    fun addAuthenticatedSite(siteUrl: URL, apiRootUrl: URL, username: String, password: String): Boolean {
        if (username.isNotEmpty() && password.isNotEmpty()) {
            authenticatedSites[AuthenticatedSite(name = siteUrl.toString(), apiRootUrl)] =
                wpAuthenticationFromUsernameAndPassword(username, password)
            return true
        }
        return false
    }

    fun authenticatedSiteList(): List<AuthenticatedSite> =
        authenticatedSites.keys.toList().sortedBy { it.name }

    fun authenticationForSite(authenticatedSite: AuthenticatedSite): WpAuthentication? =
        authenticatedSites[authenticatedSite]
}