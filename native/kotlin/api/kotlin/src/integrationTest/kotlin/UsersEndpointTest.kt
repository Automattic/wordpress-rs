package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.wp_api.SparseUserField
import uniffi.wp_api.UserId
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class UsersEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val networkHandler = WpNetworkHandler()
    private val requestHandler = WpRequestHandler(networkHandler)
    private val usersEndpoint = WPUsersEndpoint(requestHandler, apiHelper = WpApiHelper(siteUrl, authentication))

    @Test
    fun testUserListRequest() = runTest {
        val result = usersEndpoint.listWithEditContext(params = null)
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(3, userList.count())
        assertEquals("test@example.com", userList.first().email)
    }

    @Test
    fun testFilterUserListRequest() = runTest {
        val result = usersEndpoint.filterListUsers(
            WpContext.EDIT,
            params = null,
            fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
        )
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(3, userList.count())
        assertEquals("test@example.com", userList.first().email)
        assertNull(userList.first().slug)
    }

    @Test
    fun testFilterRetrieveUserRequest() = runTest {
        val result = usersEndpoint.filterRetrieveUser(
            1 as UserId,
            WpContext.EDIT,
            fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
        )
        assert(result is WpRequestSuccess)
        val sparseUser = (result as WpRequestSuccess).data
        assertEquals("test@example.com", sparseUser.email)
        assertNull(sparseUser.slug)
    }
}
