package rs.wordpress.example.shared.repository

import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class AuthenticationRepository(
    localTestSiteUrl: String,
    localTestSiteUsername: String,
    localTestSitePassword: String
) {
    private val authenticatedSites = mutableMapOf<String, WpAuthentication>()

    init {
        addAuthenticatedSite(localTestSiteUrl, localTestSiteUsername, localTestSitePassword)
    }

    fun addAuthenticatedSite(siteUrl: String, username: String, password: String): Boolean {
        if (siteUrl.isNotEmpty() && username.isNotEmpty() && password.isNotEmpty()) {
            authenticatedSites[siteUrl] = wpAuthenticationFromUsernameAndPassword(username, password)
            return true
        }
        return false
    }

    fun authenticatedSiteList(): List<String> = authenticatedSites.keys.toList().sorted()
}