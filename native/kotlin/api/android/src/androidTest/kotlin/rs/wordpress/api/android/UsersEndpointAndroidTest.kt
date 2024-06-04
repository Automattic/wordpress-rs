package rs.wordpress.api.android

import kotlinx.coroutines.test.runTest
import org.junit.Assert
import org.junit.Test
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestSuccess
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

private const val FIRST_USER_EMAIL = "test@example.com"
private const val NUMBER_OF_USERS = 3

class UsersEndpointAndroidTest {
    private val siteUrl = BuildConfig.TEST_SITE_URL
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = BuildConfig.TEST_ADMIN_USERNAME,
        password = BuildConfig.TEST_ADMIN_PASSWORD
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testUserListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = null)
        }
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        Assert.assertEquals(NUMBER_OF_USERS, userList.count())
        Assert.assertEquals(FIRST_USER_EMAIL, userList.first().email)
    }
}
