package rs.wordpress.example.shared.repository

import rs.wordpress.example.shared.domain.AuthenticatedSite
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.AccountRepository
import java.net.URI
import java.net.URL

class AuthenticationRepository(
    private val accountRepository: AccountRepository?,
    private val localTestSiteUrl: String,
    private val localTestSiteUsername: String?,
    private val localTestSitePassword: String?
) {
    // In-memory fallback for Desktop (where AccountRepository is unavailable)
    private val inMemorySites = mutableMapOf<AuthenticatedSite, WpAuthentication>()

    fun addTestSiteIfAvailable() {
        if (localTestSiteUsername != null && localTestSitePassword != null) {
            if (accountRepository != null) {
                val apiRoot = "$localTestSiteUrl/wp-json"
                val existing = accountRepository.all()
                if (existing.any { accountSiteApiRoot(it) == apiRoot }) return
            }
            addAuthenticatedSite(
                URI(localTestSiteUrl).toURL(),
                URI("$localTestSiteUrl/wp-json").toURL(),
                localTestSiteUsername,
                localTestSitePassword
            )
        }
    }

    fun addAuthenticatedSite(siteUrl: URL, apiRootUrl: URL, username: String, password: String): Boolean {
        if (username.isEmpty() || password.isEmpty()) return false

        if (accountRepository != null) {
            val account = Account.SelfHostedSite(
                id = 0UL,
                domain = siteUrl.toString(),
                username = username,
                password = password,
                siteApiRoot = apiRootUrl.toString()
            )
            accountRepository.store(account)
            return true
        }

        inMemorySites[AuthenticatedSite(name = siteUrl.toString(), apiRootUrl = apiRootUrl)] =
            wpAuthenticationFromUsernameAndPassword(username, password)
        return true
    }

    fun addWpComAccount(token: String, siteApiRoot: String, displayName: String = "WordPress.com"): Boolean {
        if (token.isEmpty()) return false

        if (accountRepository != null) {
            val account = Account.WpCom(
                id = 0UL,
                username = displayName,
                token = token,
                siteApiRoot = siteApiRoot
            )
            accountRepository.store(account)
            return true
        }

        val apiRootUrl = if (siteApiRoot.isNotEmpty()) {
            URI(siteApiRoot).toURL()
        } else {
            URI("https://public-api.wordpress.com/wp/v2").toURL()
        }
        inMemorySites[AuthenticatedSite(name = displayName, apiRootUrl = apiRootUrl)] =
            WpAuthentication.Bearer(token)
        return true
    }

    fun authenticatedSiteList(): List<AuthenticatedSite> {
        if (accountRepository != null) {
            return accountRepository.all().map { account ->
                AuthenticatedSite(
                    id = account.id(),
                    name = accountDisplayName(account),
                    apiRootUrl = URI(accountSiteApiRoot(account)).toURL()
                )
            }.sortedBy { it.name }
        }
        return inMemorySites.keys.toList().sortedBy { it.name }
    }

    fun authenticationForSite(authenticatedSite: AuthenticatedSite): WpAuthentication? {
        if (accountRepository != null) {
            val account = accountRepository.get(authenticatedSite.id) ?: return null
            return when (account) {
                is Account.SelfHostedSite ->
                    wpAuthenticationFromUsernameAndPassword(account.username, account.password)
                is Account.WpCom ->
                    WpAuthentication.Bearer(account.token)
            }
        }
        return inMemorySites[authenticatedSite]
    }

    private fun accountDisplayName(account: Account): String = when (account) {
        is Account.SelfHostedSite -> account.domain
        is Account.WpCom -> account.username
    }

    private fun accountSiteApiRoot(account: Account): String = when (account) {
        is Account.SelfHostedSite -> account.siteApiRoot
        is Account.WpCom -> account.siteApiRoot
    }
}
