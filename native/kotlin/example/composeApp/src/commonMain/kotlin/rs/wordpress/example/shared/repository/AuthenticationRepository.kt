package rs.wordpress.example.shared.repository

import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestSuccess
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class AuthenticationRepository {
    suspend fun addAuthenticatedSite(siteUrl: String, username: String, password: String): UserWithEditContext {
        val client = WpApiClient(
            siteUrl = siteUrl,
            authentication = wpAuthenticationFromUsernameAndPassword(username, password)
        )

        val user = client.request { requestBuilder ->
            requestBuilder.users().retrieveMeWithEditContext()
        } as WpRequestSuccess
        return user.data
    }
}