/*
 * This Kotlin source file was generated by the Gradle 'init' task.
 */
package rs.wordpress.api.android

import org.junit.Assert
import org.junit.Test
import rs.wordpress.api.kotlin.WPUsersEndpoint
import rs.wordpress.api.kotlin.WpNetworkHandler
import rs.wordpress.api.kotlin.WpRequestHandler
import rs.wordpress.api.kotlin.WpRequestSuccess
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class UsersEndpointAndroidTest {
    private val siteUrl = BuildConfig.TEST_SITE_URL
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = BuildConfig.TEST_ADMIN_USERNAME,
        password = BuildConfig.TEST_ADMIN_PASSWORD
    )
    private val requestHandler = WpRequestHandler(networkHandler = WpNetworkHandler())

    @Test
    fun testUserListRequest() {
        val usersEndpoint = WPUsersEndpoint(requestHandler, apiHelper = WpApiHelper(siteUrl, authentication))
        val result = usersEndpoint.listWithEditContext(params = null)
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        Assert.assertEquals(3, userList.count())
        Assert.assertEquals("test@example.com", userList.first().email)
    }
}
