package rs.wordpress.example.shared.repository

import rs.wordpress.example.shared.domain.AuthenticatedSite
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class AuthenticationRepository(
    localTestSiteUrl: String,
    localTestSiteUsername: String,
    localTestSitePassword: String
) {
    private val authenticatedSites = mutableMapOf<AuthenticatedSite, WpAuthentication>()

    init {
        addAuthenticatedSite(
            ParsedUrl.parse(localTestSiteUrl),
            ParsedUrl.parse("$localTestSiteUrl/wp-json"),
            localTestSiteUsername,
            localTestSitePassword
        )
    }

    fun addAuthenticatedSite(siteUrl: ParsedUrl, apiRootUrl: ParsedUrl, username: String, password: String): Boolean {
        if (username.isNotEmpty() && password.isNotEmpty()) {
            authenticatedSites[AuthenticatedSite(name = siteUrl.url(), apiRootUrl)] =
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